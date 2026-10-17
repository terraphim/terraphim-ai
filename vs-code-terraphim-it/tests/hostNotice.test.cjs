// Actual browser DOM/layout test: no substituted DOM or engine.
const {test} = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const fs = require('node:fs/promises');
const {chromium} = require('playwright-core');
test('recoverable notice leaves editing available, dismisses accessibly, and cannot dismiss blocking states', async () => {
  const output = process.env.TERRAPHIM_NOTICE_OUTPUT;
  if (output) { assert.ok(path.isAbsolute(output)); await fs.mkdir(output); }
  const browser = await chromium.launch({headless:true,
    ...(process.env.TERRAPHIM_NOTICE_BROWSER ? {executablePath:process.env.TERRAPHIM_NOTICE_BROWSER} : {})});
  try {
    const page = await browser.newPage({viewport:{width:1000,height:700}});
    await page.setContent('<div id="host-notice" role="alert" hidden></div><div id="editor"><textarea aria-label="Markdown source" style="position:absolute;top:350px;left:100px"></textarea></div>');
    await page.addStyleTag({path:process.env.TERRAPHIM_NOTICE_STYLE || path.resolve(__dirname,'../media/alternative-control.css')});
    await page.addScriptTag({path:path.resolve(__dirname,'../media/host-notice.js')});
    await page.evaluate(()=>{
      globalThis.actualNotice = new TerraphimHostNotice(document.getElementById('host-notice'));
      actualNotice.recoverable('Terraphim: Document changed before this operation');
    });
    const alert = page.getByRole('alert');
    const bounds = await alert.boundingBox();
    assert.ok(bounds.height < 175, 'recoverable status must not cover the editor');
    if (output) await page.screenshot({path:path.join(output,'recoverable-notice.png')});
    await page.getByRole('textbox',{name:'Markdown source'}).click();
    await page.getByRole('textbox',{name:'Markdown source'}).pressSequentially('Actual input after recoverable rejection');
    assert.equal(await page.getByRole('textbox',{name:'Markdown source'}).inputValue(),'Actual input after recoverable rejection');
    await page.getByRole('button',{name:'Dismiss Terraphim notice'}).click();
    assert.equal(await alert.isVisible(),false);
    if (output) await page.screenshot({path:path.join(output,'dismissed-notice.png')});
    await page.evaluate(()=>{
      actualNotice.recoverable('Temporary notice');
      globalThis.oldDismiss = document.querySelector('[data-dismiss-notice]');
      actualNotice.blocking();
      document.getElementById('host-notice').replaceChildren(document.createTextNode('A recovery draft is available'));
      document.getElementById('editor').inert = true;
      oldDismiss.click(); // An obsolete notice control cannot hide a later blocking state.
    });
    assert.equal(await alert.isVisible(),true);
    assert.equal(await page.getByRole('button',{name:'Dismiss Terraphim notice'}).count(),0);
    assert.equal(await page.locator('#editor').evaluate(element=>element.inert),true);
    const blocking = await alert.boundingBox();
    assert.ok(blocking.height > 600, 'blocking recovery/error presentation must remain intact');
    if (output) {
      await page.screenshot({path:path.join(output,'blocking-recovery.png')});
      await fs.writeFile(path.join(output,'receipt.json'),JSON.stringify({status:'PASS',scope:'Actual headless Chromium DOM/layout; not native host qualification',browser:browser.version(),recoverableBounds:bounds,blockingBounds:blocking,sourceAfterDismiss:await page.locator('textarea').inputValue(),blockingRemainsVisible:await alert.isVisible(),dismissControls:await page.getByRole('button',{name:'Dismiss Terraphim notice'}).count()},null,2)+'\n');
    }
  } finally { await browser.close(); }
});
