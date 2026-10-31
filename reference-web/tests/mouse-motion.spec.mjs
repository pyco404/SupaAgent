import {test,expect} from '../../web/node_modules/@playwright/test/index.mjs';

// Read the actual rendered mouse region; keyboard keys lie to its left.
async function mouseCenter(canvas){
 return canvas.evaluate(el=>{
  const [left,top,width]=el.dataset.keyboardBounds.split(',').map(Number),scale=el.width/el.getBoundingClientRect().width;
  const x=Math.ceil((left+width)*scale),y=Math.max(0,Math.floor(top*scale));
  const pixels=el.getContext('2d').getImageData(x,y,el.width-x,el.height-y);let weight=0,cx=0,cy=0;
  for(let row=0;row<pixels.height;row++)for(let col=0;col<pixels.width;col++){const alpha=pixels.data[(row*pixels.width+col)*4+3];weight+=alpha;cx+=(col+x)*alpha;cy+=(row+y)*alpha;}
  return {x:cx/weight/scale,y:cy/weight/scale,weight};
 });
}
async function move(page,canvas,x,y){
 await page.mouse.move(x,y);await expect(canvas).toHaveAttribute('data-right-hand','mouse');
 await page.waitForTimeout(90);return mouseCenter(canvas);
}

test('rendered mouse and gripping hand follow pointer direction in intro and workspace',async({page})=>{
 await page.emulateMedia({reducedMotion:'no-preference'});
 const {width,height}=page.viewportSize();
 for(const [route,selector] of [['/','.intro-canvas'],['/about','.workspace-hands']]){
  await page.goto(route);await expect(page.locator('html')).toHaveAttribute('data-ready','true');const canvas=page.locator(selector);
  const start=await move(page,canvas,width*.15,height*.3),bounds=await canvas.getAttribute('data-keyboard-bounds');
  const end=await move(page,canvas,width*.85,height*.65);
  expect(start.weight).toBeGreaterThan(0);expect(end.x-start.x).toBeGreaterThan(1);expect(end.y-start.y).toBeGreaterThan(.5);
  expect(await canvas.getAttribute('data-keyboard-bounds')).toBe(bounds);
  const back=await move(page,canvas,width*.15,height*.3);expect(back.x).toBeLessThan(end.x-1);expect(back.y).toBeLessThan(end.y-.5);
  await expect(canvas).toHaveAttribute('data-left-hand','keyboard');await expect(canvas).toHaveAttribute('data-right-hand','keyboard');
 }
});

test('reduced motion keeps mouse position still and touch never activates the mouse hand',async({page})=>{
 await page.goto('/about');await expect(page.locator('html')).toHaveAttribute('data-ready','true');const canvas=page.locator('.workspace-hands'),{width,height}=page.viewportSize();
 const before=await move(page,canvas,width*.2,height*.3),after=await move(page,canvas,width*.8,height*.7);
 expect(after.x).toBeCloseTo(before.x,1);expect(after.y).toBeCloseTo(before.y,1);
 await page.evaluate(()=>window.dispatchEvent(new PointerEvent('pointermove',{clientX:120,clientY:180,pointerType:'touch'})));
 await expect(canvas).toHaveAttribute('data-right-hand','keyboard');
});
