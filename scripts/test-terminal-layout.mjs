/** Local-only layout capture and regression checks. No backend or mutation calls. */
import {chromium} from '../web/node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';

const baseline=process.argv.includes('--before');
const phase=baseline?'before':'after',origin='http://127.0.0.1:3215';
const output=new URL('../artifacts/terminal-spacing/',import.meta.url);
await mkdir(output,{recursive:true});
const sizes=[['desktop',1440,1000],['laptop',1366,768],['compact',1280,600],['tablet-landscape',1024,768],['tablet',768,1024],['mobile',390,844],['small-mobile',375,667],['landscape',844,390]];
const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true});
const results=[];
try{
 for(const [name,width,height]of sizes){
  const context=await browser.newContext({viewport:{width,height},reducedMotion:'reduce'});
  const page=await context.newPage(),errors=[],blocked=[];
  page.on('pageerror',e=>errors.push(e.name));
  await context.route('**/*',route=>route.abort());
  await page.route('**/*',async route=>{
   const request=route.request(),url=new URL(request.url());
   if(url.origin!==origin||!['GET','HEAD'].includes(request.method())||url.pathname.startsWith('/api/')){blocked.push(url.pathname);return route.abort();}
   const response=await route.fetch();
   if(response.headers()['content-type']?.includes('text/html'))return route.fulfill({response,body:(await response.text()).replace('data-motion="running"','data-motion="running" data-public-landing="true"')});
   return route.fulfill({response});
  });
  for(const section of ['agents','factory']){
   await page.goto(origin+'/'+section);
   await page.locator('html[data-ready=true]').waitFor();
   await page.locator('.section-image').evaluate(image=>image.decode());
   await page.waitForLoadState('networkidle');
   const measurements=await page.evaluate(()=>{
    const box=selector=>{const r=document.querySelector(selector).getBoundingClientRect();return{x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom};};
    const buffer=document.querySelector('.buffer'),style=getComputedStyle(buffer);
    return{wheel:box('.wheel'),pane:box('.pane'),entity:box('.entity'),buffer:box('.buffer'),footer:box('.bar'),keyboard:box('.workspace-hands'),artwork:box('.section-composition'),horizontalOverflow:document.documentElement.scrollWidth>innerWidth,scrollHeight:buffer.scrollHeight,clientHeight:buffer.clientHeight,overflowY:style.overflowY};
   });
   await page.screenshot({path:new URL(`${phase}-${name}-${section}.png`,output).pathname});
   await page.locator('.buffer').evaluate(el=>{el.scrollTop=el.scrollHeight;});
   const last=await page.locator('.buffer > :last-child').boundingBox();
   const row={viewport:name,width,height,section,...measurements,lastContentBottom:last.y+last.height,blocked:[...blocked],errors:[...errors]};results.push(row);
   if(section==='factory')await page.screenshot({path:new URL(`${phase}-${name}-factory-end.png`,output).pathname});
   if(!baseline){
    assert.equal(measurements.horizontalOverflow,false,name+'_horizontal_overflow');
    assert.ok(measurements.buffer.height>=80,name+'_usable_terminal_height');
    assert.ok(measurements.buffer.bottom<=measurements.footer.y-12,name+'_footer_clearance');
    assert.ok(measurements.keyboard.y>=measurements.footer.bottom,name+'_keyboard_clearance');
    assert.ok(last.y+last.height<=measurements.buffer.bottom,name+'_last_content_reachable');
    assert.ok(['auto','scroll'].includes(measurements.overflowY));
    assert.equal(errors.length,0);assert.equal(blocked.length,0);
    if(width>700){
     const firstGap=measurements.pane.x-measurements.wheel.right,secondGap=measurements.entity.x-measurements.pane.right;
     assert.ok(firstGap>=16&&secondGap>=16,name+'_column_separation');
     assert.ok(Math.abs(firstGap-secondGap)<1,name+'_equal_gutters');
     assert.ok(measurements.pane.width<=621,name+'_bounded_reading_width');
     assert.ok(Math.abs(measurements.wheel.y-measurements.pane.y)<1&&Math.abs(measurements.entity.y-measurements.pane.y)<1,name+'_top_alignment');
    }
    if(section==='factory'){assert.ok(measurements.scrollHeight>measurements.clientHeight);assert.equal(await page.locator('.buffer button:disabled').count(),4);assert.equal(await page.locator('.buffer form').count(),0);}
   }
  }
  if(!baseline&&width<=700){await page.locator('.tabline').click();assert.equal(await page.locator('.wheel').isVisible(),true);await page.keyboard.press('Escape');assert.equal(await page.locator('.wheel').isVisible(),false);}
  await page.waitForLoadState('networkidle');await page.unrouteAll({behavior:'wait'});await context.close();
 }
 if(!baseline){
  const context=await browser.newContext();
  for(const path of ['/operator','/my-agents','/api/operator-status','/api/agents/11111111-1111-4111-8111-111111111111/memory'])assert.equal((await context.request.get(origin+path)).status(),401,path+'_protected');
  await context.close();
 }
 await writeFile(new URL(`${phase}-measurements.json`,output),JSON.stringify({passed:true,phase,results},null,2)+'\n');
 console.log(JSON.stringify({passed:true,phase,viewportCount:sizes.length,sectionChecks:results.length,privateRequests:results.reduce((n,r)=>n+r.blocked.length,0)}));
}catch(error){await writeFile(new URL(`${phase}-measurements.json`,output),JSON.stringify({passed:false,phase,error:String(error.message).slice(0,300),results},null,2)+'\n');throw error;}
finally{await browser.close();}
