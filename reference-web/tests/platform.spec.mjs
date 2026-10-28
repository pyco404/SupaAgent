import { test, expect } from '../../web/node_modules/@playwright/test/index.mjs';
const id='11111111-1111-4111-8111-111111111111';
const connected='http://127.0.0.1:3202';
const signIn={username:'fixture',password:'fixture-password-'.repeat(3)};
async function openUtilities(page){if(await page.locator('.command').isHidden())await page.locator('.command-toggle').click();}
async function showAll(page){await openUtilities(page);await page.locator('.skip-typing').click();}
async function reloadSaved(page){await openUtilities(page);await page.locator('.reload-records').click();}


async function go(page,section){await page.goto(`${connected}/site/#${section}`,{waitUntil:'domcontentloaded'});await expect(page.locator('html')).toHaveAttribute('data-ready','true');await expect(page.locator('#agent-select')).toHaveValue(id);}

test('public identities are minimal and private endpoints refuse unauthenticated reads',async({request})=>{
  const response=await request.get(`${connected}/api/agents`);expect(response.status()).toBe(200);
  const agents=await response.json();expect(Object.keys(agents[0]).sort()).toEqual(['created_at','id','lifecycle','name']);
  for(const part of ['memory','goals','financials','social','wallet/balances'])expect((await request.get(`${connected}/api/agents/${id}/${part}`)).status()).toBe(401);
  for(const part of ['pause','resume','wallet/sign','cycles'])expect((await request.post(`${connected}/api/agents/${id}/${part}`)).status()).toBe(405);
  expect((await request.get(`${connected}/api/agents`,{headers:{Origin:'https://outside.invalid'}})).status()).toBe(403);
});
test('public visitors see protected states without receiving private memory',async({page})=>{
  const privateResponses=[];page.on('response',response=>{if(response.url().endsWith('/memory'))privateResponses.push(response.status());});
  await go(page,'memory');await expect(page.locator('.buffer')).toContainText('Operator sign-in required');
  await expect(page.locator('.buffer')).not.toContainText('Browser-test hypothesis');expect(privateResponses).toContain(401);
});

test.describe('authorized saved records',()=>{
  test.use({httpCredentials:signIn});
  test.beforeEach(async({page})=>{await page.goto(`${connected}/operator`);await expect(page).toHaveURL(/\/site\/#agents$/);});
  test('all ten sections use honest data and planned capability labels',async({page})=>{
    const checks={home:'Born to think. Built to survive.',agents:'Fixture Supa',intelligence:'authorization allowed',memory:'Hypothesis / unverified',economy:'Simulated ledger / hypothetical USD',social:'TEST FIXTURE: My simulated budget',trading:'strict maximum of $5', 'token-launch':'Pump.fun token launching is planned',factory:'Creation requires explicit authorization',about:'six-crate Rust runtime'};
    for(const [section,text]of Object.entries(checks)){
      await go(page,section);await expect(page.locator('.buffer')).toContainText(text);
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
      await expect(page.locator('body')).not.toContainText(/HANCORE|Shibumi|Omarchy|CAPTURED GITHUB/);
    }
    expect(await page.locator('.wheel-item').count()).toBe(10);
  });
  test('economy separates simulated values, mock Devnet evidence and retrieval time',async({page})=>{
    await go(page,'economy');const buffer=page.locator('.buffer');
    await expect(buffer).toContainText('2.5');await expect(buffer).toContainText('Mainnet / unavailable');await expect(buffer).toContainText('No saved wallet balance observation');await expect(buffer).toContainText('Observation time  unavailable');
    await buffer.getByText('Runway calculation',{exact:true}).click();await expect(buffer).toContainText('Insufficient recorded simulated expense history.');
  });
  test('agent selection, terminal history and escaped record text are interactive',async({page})=>{
    await go(page,'agents');await page.locator('.terminal-entry').click();await expect(page).toHaveURL(/#intelligence$/);await expect(page.locator('.buffer')).toContainText('cycle');
    await page.locator('.command-toggle').click();await page.locator('.command-input').fill('supa memory');await page.keyboard.press('Enter');await expect(page).toHaveURL(/#memory$/);
    await page.locator('.command-toggle').click();await page.keyboard.press('ArrowUp');await expect(page.locator('.command-input')).toHaveValue('supa memory');await page.keyboard.press('Escape');
    await page.route('**/api/agents/*/memory',route=>route.fulfill({json:[{id,lesson:'<img src=x onerror=alert(1)>',kind:'hypothesis',source_event_ids:[],cycle_id:null,created_at:'fixture'}]}));
    await reloadSaved(page);await expect(page.locator('.buffer')).toContainText('<img src=x');expect(await page.locator('.buffer img').count()).toBe(0);
  });
  test('record retrieval has loading, empty, error and recovery states',async({page})=>{
    await go(page,'agents');
    await page.route('**/api/agents',async route=>{await new Promise(resolve=>setTimeout(resolve,350));await route.fulfill({json:[]});});
    await reloadSaved(page);await expect(page.locator('.reload-records')).toBeDisabled();await expect(page.locator('.buffer')).toContainText('No saved agents');
    await page.unroute('**/api/agents');await reloadSaved(page);await expect(page.locator('.buffer')).toContainText('Fixture Supa');
    await page.route('**/api/agents',route=>route.fulfill({status:503,json:{error:'sanitized'}}));await reloadSaved(page);await expect(page.locator('.buffer')).toContainText('Saved records are unavailable');
  });
  test('mobile menu traps focus, keeps selection browsing open and closes on activation',async({page})=>{
    await page.setViewportSize({width:390,height:844});await go(page,'agents');
    await page.locator('.tabline').click();await expect(page.locator('body')).toHaveClass(/menu-open/);await expect(page.locator('.pane')).toHaveJSProperty('inert',true);
    await page.keyboard.press('ArrowDown');await expect(page.locator('.tabline')).toHaveAttribute('aria-expanded','true');
    await page.locator('.wheel-item[aria-selected=true]').focus();await page.keyboard.press('Tab');await expect(page.locator('.tabline')).toBeFocused();
    await page.keyboard.press('Shift+Tab');await expect(page.locator('.wheel-item[aria-selected=true]')).toBeFocused();
    await page.keyboard.press('Enter');await expect(page.locator('.tabline')).toHaveAttribute('aria-expanded','false');await expect(page.locator('.pane')).toHaveJSProperty('inert',false);
  });
});

test('every section types in document order and replays on return',async({page})=>{
  await page.emulateMedia({reducedMotion:'no-preference'});
  await page.goto('/site/#home');
  await expect(page.locator('html')).toHaveAttribute('data-ready','true');
  await expect(page.locator('.reload-records')).toBeEnabled();
  for(const section of ['home','agents','intelligence','memory','economy','social','trading','token-launch','factory','about','home']){
    await page.locator('.command-toggle').click();await page.locator('.command-input').fill(`supa ${section}`);await page.keyboard.press('Enter');
    const buffer=page.locator('.buffer');await expect(buffer).toHaveAttribute('data-typing-state','typing');
    expect(await buffer.locator(':scope > [hidden]').count()).toBeGreaterThan(0);
    const order=await buffer.evaluate(el=>{let hidden=false;return [...el.children].every(row=>{if(row.hidden)hidden=true;return row.hidden||!hidden;});});expect(order).toBe(true);
    const before=(await buffer.textContent()).length;await page.waitForTimeout(180);expect((await buffer.textContent()).length).toBeGreaterThanOrEqual(before);
    await showAll(page);await expect(buffer).toHaveAttribute('data-typing-state','complete');
    expect(await buffer.locator('[hidden],[inert]').count()).toBe(0);await expect(page.locator('.skip-typing')).toBeHidden();await expect(buffer).toBeFocused();
    await expect(page).toHaveURL(new RegExp(`#${section}$`));
  }
});

test('reloading restarts typing and navigation cancels old text safely',async({page})=>{
  await page.emulateMedia({reducedMotion:'no-preference'});await page.goto('/site/#about');
  await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','typing');await showAll(page);
  await reloadSaved(page);await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','typing');
  await page.locator('.command-toggle').click();await page.locator('.command-input').fill('supa trading');await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/#trading$/);await showAll(page);
  await expect(page.locator('.buffer')).toContainText('strict maximum of $5');await page.waitForTimeout(300);
  await expect(page.locator('.buffer')).not.toContainText('six-crate Rust runtime');
  await page.emulateMedia({reducedMotion:'reduce'});await reloadSaved(page);
  await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','complete');await expect(page.locator('.skip-typing')).toBeHidden();
});


test('workspace stays clear and Home uses the latest source-code logo',async({page})=>{
  await page.goto('/site/#home');await expect(page.locator('html')).toHaveAttribute('data-ready','true');
  await expect(page.locator('.workspace-controls > button,.workspace-controls > a')).toHaveCount(0);
  await expect(page.locator('.wheel-item[data-index="0"] img')).toHaveAttribute('src','/public/assets/supa-agent-code.svg');
  await expect(page.locator('.reload-records')).toBeHidden();await expect(page.getByRole('link',{name:'Operator sign-in'})).toBeHidden();
  await page.getByRole('button',{name:'Open terminal commands'}).click();
  await expect(page.locator('.reload-records')).toBeVisible();await expect(page.getByRole('link',{name:'Operator sign-in'})).toBeVisible();
  await expect(page.locator('.command-input')).toBeFocused();await page.getByRole('button',{name:'Close terminal commands'}).click();await expect(page.locator('.command')).toBeHidden();
  await page.getByRole('button',{name:'Open terminal commands'}).click();await page.keyboard.press('Escape');await expect(page.locator('.command')).toBeHidden();
});

test('agent picker stays within the viewport and supports keyboard selection and dismissal',async({page})=>{
  const second='22222222-2222-4222-8222-222222222222',longName='A very long verified agent identity '.repeat(6);
  await page.route('**/api/agents',route=>route.fulfill({json:[{id,name:'Fixture Supa',lifecycle:'active',created_at:'fixture'},{id:second,name:longName,lifecycle:'paused',created_at:'fixture'}]}));
  await page.goto('/site/#home');await expect(page.locator('#agent-picker-trigger')).toBeEnabled();
  const trigger=page.locator('#agent-picker-trigger'),list=page.locator('#agent-options');
  await expect(page.locator('#agent-select')).toBeHidden();await trigger.click();await expect(list).toBeVisible();
  const inside=async()=>{const box=await list.boundingBox(),viewport=page.viewportSize();expect(box.x).toBeGreaterThanOrEqual(0);expect(box.y).toBeGreaterThanOrEqual(0);expect(box.x+box.width).toBeLessThanOrEqual(viewport.width);expect(box.y+box.height).toBeLessThanOrEqual(viewport.height);};
  await inside();expect(await list.evaluate(el=>el.scrollWidth<=el.clientWidth)).toBe(true);
  await page.locator('.buffer').evaluate(el=>el.dispatchEvent(new Event('scroll')));await expect(list).toBeVisible();
  await page.keyboard.press('j');await expect(page).toHaveURL(/#home$/);await expect(list).toBeVisible();
  await page.keyboard.press('ArrowDown');await page.keyboard.press('Enter');await expect(page.locator('#agent-select')).toHaveValue(second);await expect(list).toBeHidden();await expect(trigger).toBeFocused();await expect(trigger).toContainText(longName);
  await trigger.click();await page.keyboard.press('Escape');await expect(list).toBeHidden();await expect(trigger).toBeFocused();
  await trigger.click();await page.locator('.blueprint').click({position:{x:5,y:5}});await expect(list).toBeHidden();
  await trigger.click();await page.setViewportSize({width:360,height:640});await expect(list).toBeHidden();await trigger.click();await inside();
  await page.keyboard.press('Escape');await trigger.evaluate(el=>Object.assign(el.style,{position:'fixed',bottom:'8px',left:'20px',width:'calc(100vw - 40px)'}));await trigger.click();await inside();
  const above=await list.boundingBox(),field=await trigger.boundingBox();expect(above.y+above.height).toBeLessThanOrEqual(field.y);
});

test('agent popup remains inside a scaled embedded preview',async({page})=>{
  const original=await page.request.get(`${connected}/site/`);expect(original.headers()['content-security-policy']).toContain("frame-ancestors 'none'");
  // Only this local fixture permits embedding; production framing policy stays intact.
  await page.route(`${connected}/site/`,async route=>{const response=await route.fetch(),headers=response.headers();delete headers['content-security-policy'];await route.fulfill({response,headers});});
  await page.setViewportSize({width:800,height:900});await page.goto('about:blank');
  await page.setContent(`<iframe title="Local Supa preview" src="${connected}/site/#home" style="position:absolute;left:200px;top:40px;width:390px;height:700px;border:0;transform:scale(.75);transform-origin:top left"></iframe>`);
  const frame=page.frameLocator('iframe');await expect(frame.locator('#agent-picker-trigger')).toBeEnabled();await frame.locator('#agent-picker-trigger').click();
  const panel=await frame.locator('#agent-options').boundingBox(),preview=await page.locator('iframe').boundingBox();
  expect(panel.x).toBeGreaterThanOrEqual(preview.x);expect(panel.y).toBeGreaterThanOrEqual(preview.y);expect(panel.x+panel.width).toBeLessThanOrEqual(preview.x+preview.width);expect(panel.y+panel.height).toBeLessThanOrEqual(preview.y+preview.height);
  await frame.locator('#agent-options [role=option]').last().click();await expect(frame.locator('#agent-options')).toBeHidden();
});
