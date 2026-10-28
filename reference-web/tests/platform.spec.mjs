import { test, expect } from '../../web/node_modules/@playwright/test/index.mjs';
const id='11111111-1111-4111-8111-111111111111';
const connected='http://127.0.0.1:3202';
const signIn={username:'fixture',password:'fixture-password-'.repeat(3)};

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
    await page.locator('.reload-records').click();await expect(page.locator('.buffer')).toContainText('<img src=x');expect(await page.locator('.buffer img').count()).toBe(0);
  });
  test('record retrieval has loading, empty, error and recovery states',async({page})=>{
    await go(page,'agents');
    await page.route('**/api/agents',async route=>{await new Promise(resolve=>setTimeout(resolve,350));await route.fulfill({json:[]});});
    await page.locator('.reload-records').click();await expect(page.locator('.reload-records')).toBeDisabled();await expect(page.locator('.buffer')).toContainText('No saved agents');
    await page.unroute('**/api/agents');await page.locator('.reload-records').click();await expect(page.locator('.buffer')).toContainText('Fixture Supa');
    await page.route('**/api/agents',route=>route.fulfill({status:503,json:{error:'sanitized'}}));await page.locator('.reload-records').click();await expect(page.locator('.buffer')).toContainText('Saved records are unavailable');
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
    await page.locator('.skip-typing').click();await expect(buffer).toHaveAttribute('data-typing-state','complete');
    expect(await buffer.locator('[hidden],[inert]').count()).toBe(0);await expect(page.locator('.skip-typing')).toBeHidden();await expect(buffer).toBeFocused();
    await expect(page).toHaveURL(new RegExp(`#${section}$`));
  }
});

test('reloading restarts typing and navigation cancels old text safely',async({page})=>{
  await page.emulateMedia({reducedMotion:'no-preference'});await page.goto('/site/#about');
  await expect(page.locator('.skip-typing')).toBeVisible();await page.locator('.skip-typing').click();
  await page.locator('.reload-records').click();await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','typing');
  await page.locator('.command-toggle').click();await page.locator('.command-input').fill('supa trading');await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/#trading$/);await page.locator('.skip-typing').click();
  await expect(page.locator('.buffer')).toContainText('strict maximum of $5');await page.waitForTimeout(300);
  await expect(page.locator('.buffer')).not.toContainText('six-crate Rust runtime');
  await page.emulateMedia({reducedMotion:'reduce'});await page.locator('.reload-records').click();
  await expect(page.locator('.buffer')).toHaveAttribute('data-typing-state','complete');await expect(page.locator('.skip-typing')).toBeHidden();
});
