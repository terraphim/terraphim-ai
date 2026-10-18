'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { HistoryCommands } = require('../out/historyCommands');

test('a second host-issued command cannot supersede or execute before the first terminal completion', () => {
 const commands = new HistoryCommands();
 const first = commands.issue(7);
 const second = commands.issue(7);
 assert.match(first.nonce, /^[0-9a-f-]{36}$/);
 assert.notEqual(first.nonce, second.nonce);
 assert.equal(commands.matchesHead(first.nonce, 7, 7), true);
 assert.equal(commands.matchesHead(second.nonce, 7, 7), false);
 assert.equal(commands.completeHead(second.nonce, 7, 7), false);
 assert.equal(commands.size, 2);
 assert.equal(commands.matchesHead(first.nonce, 7, 7), true);
 assert.equal(commands.completeHead(first.nonce, 7, 7), true);
 assert.equal(commands.matchesHead(second.nonce, 7, 7), true);
 assert.equal(commands.completeHead(first.nonce, 7, 7), false);
 assert.equal(commands.completeHead(second.nonce, 7, 7), true);
 assert.equal(commands.size, 0);
});

test('future, foreign and stale view completions retain the actual executing command', () => {
 const commands = new HistoryCommands();
 const head = commands.issue(0);
 const future = commands.issue(0);
 for (const [nonce, epoch, current] of [[future.nonce, 0, 0], [head.nonce, 1, 1], [head.nonce, 0, 1], ['foreign', 0, 0], [head.nonce, NaN, 0], [undefined, 0, 0]]) {
  assert.equal(commands.matchesHead(nonce, epoch, current), false);
  assert.equal(commands.completeHead(nonce, epoch, current), false);
  assert.equal(commands.size, 2);
 }
 assert.equal(commands.completeHead(head.nonce, 0, 0), true);
 assert.equal(commands.matchesHead(future.nonce, 0, 0), true);
});

test('an activation or authority reset retires every prior command without admitting a late completion', () => {
 const commands = new HistoryCommands();
 const old = commands.issue(5);
 commands.issue(5);
 commands.clear();
 const fresh = commands.issue(6);
 assert.equal(commands.completeHead(old.nonce, 5, 6), false);
 assert.equal(commands.matchesHead(fresh.nonce, 6, 6), true);
 commands.clear();
 const recovered = commands.issue(6);
 assert.equal(commands.completeHead(fresh.nonce, 6, 6), false);
 assert.equal(commands.matchesHead(recovered.nonce, 6, 6), true);
});

test('the queue is bounded at32 and a refused issue never replaces accepted tokens', () => {
 const commands = new HistoryCommands();
 const issued = Array.from({ length: 32 }, () => commands.issue(12));
 assert.throws(() => commands.issue(12), /Too many pending/);
 assert.equal(commands.size, 32);
 for (const item of issued) {
  assert.equal(commands.matchesHead(item.nonce, 12, 12), true);
  assert.equal(commands.completeHead(item.nonce, 12, 12), true);
 }
 assert.equal(commands.size, 0);
 for (const invalid of [-1, NaN, Infinity, 0.5, Number.MAX_SAFE_INTEGER + 1]) assert.throws(() => commands.issue(invalid), /Invalid history view epoch/);
 assert.equal(commands.size, 0);
});

test('real issued FIFO drain waits for every actual head, including commands issued while waiting',async()=>{
 const queue=new HistoryCommands(),first=queue.issue(4);let settled=false;
 const draining=queue.waitForIdle(4).then(()=>{settled=true;});const second=queue.issue(4);
 assert.equal(queue.completeHead(second.nonce,4,4),false);await Promise.resolve();assert.equal(settled,false);
 assert.equal(queue.completeHead(first.nonce,4,4),true);await Promise.resolve();assert.equal(settled,false);
 assert.equal(queue.completeHead(second.nonce,4,4),true);await draining;assert.equal(settled,true);assert.equal(queue.size,0);
});
test('actual epoch retirement rejects a pending history drain instead of acknowledging completion',async()=>{
 const queue=new HistoryCommands();queue.issue(2);const draining=queue.waitForIdle(2);const refused=assert.rejects(draining,/retired/);queue.clear();await refused;
 const fresh=queue.issue(3);await assert.rejects(queue.waitForIdle(2),/epoch/);assert.equal(queue.completeHead(fresh.nonce,3,3),true);await queue.waitForIdle(3);
});
test('real timer bounds a stuck title drain without consuming its command',async()=>{
 const queue=new HistoryCommands(),head=queue.issue(1);await assert.rejects(queue.waitForIdle(1,20),/did not settle/);assert.equal(queue.size,1);assert.equal(queue.completeHead(head.nonce,1,1),true);await queue.waitForIdle(1);
});
test('invalid drain boundaries are refused without changing real FIFO state',async()=>{
 const queue=new HistoryCommands();queue.issue(0);for(const [epoch,timeout] of [[-1,1],[0,0],[0,15001],[0,Infinity]])await assert.rejects(queue.waitForIdle(epoch,timeout),/Invalid/);assert.equal(queue.size,1);queue.clear();
});
