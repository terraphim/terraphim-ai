# Implementation Routing

Code implementation, bug fixes, refactoring, feature development, and PR creation.
The workhorse routing for most coding tasks. Needs fast, cost-effective models
with strong code generation and Rust expertise.

priority:: 50

synonyms:: implement, build, code, fix, refactor, feature, PR, coding task,
  implementation swarm, new feature, bug fix, patch, enhancement, migration,
  scaffold, boilerplate, cargo build, compilation fix, lint fix

trigger:: code implementation and feature development tasks in Rust

route:: kimi, kimi-for-coding/k2p5
action:: opencode -m kimi-for-coding/k2p5 -p "{{ prompt }}"

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model claude-sonnet-4-6 -p "{{ prompt }}" --max-turns 50
