import { test, expect } from "../../web/node_modules/@playwright/test/index.mjs";
import { readFileSync } from "node:fs";

const navigationFocus = page => page.locator(page.viewportSize().width <= 700 ? "button.tabline" : ".wheel");
async function open(page, path) {
  // Remote preview art is optional; interaction readiness comes from the local app.
  await page.goto(path, { waitUntil: "domcontentloaded" });
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
}

test("intro enters the project browser and browser back restores intro", async ({ page }) => {
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await open(page, "/");
  await expect(page.locator("body")).toHaveAttribute("data-view", "intro");
  await expect(page.locator("a.enter")).toHaveAttribute("href", "/site/");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/site\/(?:#.*)?$/);
  await expect(page.locator("body")).toHaveAttribute("data-view", "projects");
  await expect(page.locator(".bar")).toBeVisible();
  await page.goBack();
  await expect(page.locator("body")).toHaveAttribute("data-view", "intro");
  expect(errors).toEqual([]);
});

test("project browser supports keyboard selection and Escape focus", async ({ page }) => {
  await open(page, "/site/");
  const options = page.getByRole("option", { includeHidden: true });
  await expect(options.first()).toBeAttached();
  const selected = () => page.locator('.wheel-item[aria-selected="true"]');
  const initial = await selected().getAttribute("data-index");
  if (page.viewportSize().width <= 700) await page.locator("button.tabline").click();
  await page.locator(".wheel").focus();
  await page.keyboard.press("ArrowDown");
  await expect.poll(() => selected().getAttribute("data-index")).not.toBe(initial);
  await page.keyboard.press("k");
  await expect(selected()).toHaveAttribute("data-index", initial);
  await page.keyboard.press("Escape");
  await expect(navigationFocus(page)).toBeFocused();
});

test("terminal scrolling keys preserve the selected project", async ({ page }) => {
  await open(page, "/site/");
  const selected = page.locator('.wheel-item[aria-selected="true"]');
  const initial = await selected.getAttribute("data-index");
  await page.locator(".buffer").focus();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("j");
  await expect(selected).toHaveAttribute("data-index", initial);
  await expect(page.locator(".buffer")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(navigationFocus(page)).toBeFocused();
});

test("command input is keyboard accessible and does not navigate externally", async ({ page }) => {
  await open(page, "/site/");
  await expect(page.locator(".wheel-item").first()).toBeAttached();
  await page.locator(".buffer").focus();
  await page.keyboard.press(":");
  await expect(page.locator(".command-input")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(navigationFocus(page)).toBeFocused();
  await expect(page).toHaveURL(/\/site\/(?:#.*)?$/);
  await page.keyboard.press(":");
  await page.locator(".command-input").fill("globalThis.__supaCommandExecuted=true");
  await page.keyboard.press("Enter");
  await expect(page.locator(".announcement")).toContainText("Unknown command");
  expect(await page.evaluate(() => globalThis.__supaCommandExecuted)).toBeUndefined();
});

test("Supa hashes and navigational commands restore saved selections", async ({ page }) => {
  await open(page, "/site/#economy");
  await expect(page.locator('.wheel-item[aria-selected="true"]')).toHaveAttribute("data-index", "4");
  await page.locator(".buffer").focus();await page.keyboard.press(":");
  await page.locator(".command-input").fill("supa social");await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/#social$/);
  await expect(page.locator(".buffer")).toContainText("Select an agent");
  await page.reload({waitUntil:"domcontentloaded"});
  await expect(page.locator('.wheel-item[aria-selected="true"]')).toHaveAttribute("aria-label", "Social");
});

test("dark palette survives reload and live motion preference changes are respected", async ({ page }) => {
  await open(page, "/");
  const background = () => page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--bg").trim());
  const original = await background();
  await page.locator(".theme-key").click();
  await expect.poll(background).not.toBe(original);
  const chosen = await background();
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  expect(await background()).toBe(chosen);
  const rgb = await page.locator("body").evaluate(element => getComputedStyle(element).backgroundColor);
  expect(rgb.match(/[\d.]+/g).slice(0, 3).every(value => Number(value) < 60)).toBe(true);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await expect(page.locator("html")).toHaveAttribute("data-motion", "running");
  // Commit a rendered motion frame before changing the emulated preference again.
  // Chromium can coalesce back-to-back emulation changes in the same paint cycle.
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator("html")).toHaveAttribute("data-motion", "still");
});

test("normal terminal boot restores complete accessible text when motion is reduced", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/site/", { waitUntil: "domcontentloaded" });
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  const buffer = page.locator(".buffer");
  await expect(buffer).toHaveAttribute("aria-busy", "true");
  await expect(buffer).toHaveAttribute("aria-live", "off");
  await expect(page.locator(".entity-state b").nth(1)).toHaveText("typing");
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(buffer).toHaveAttribute("aria-busy", "false");
  await expect(buffer).toHaveAttribute("aria-live", "polite");
  await expect(buffer).toContainText("Born to think. Built to survive.");
  expect(await buffer.locator("[hidden]").count()).toBe(0);
  await expect(page.locator(".entity-state b").nth(1)).toHaveText("idle");
});

test("opening code types slowly from the top and reduced motion completes it", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await open(page, "/");
  const intro = page.locator(".intro-canvas");
  await page.waitForTimeout(1200);
  await expect(intro).toHaveAttribute("data-intro-state", "typing");
  const first = await intro.evaluate(canvas => ({ rows: Number(canvas.dataset.revealedRows), characters: Number(canvas.dataset.typedCharacters) }));
  expect(first.characters).toBeGreaterThan(0);
  expect(first.rows).toBeLessThan(8);
  await page.waitForTimeout(1400);
  const next = await intro.evaluate(canvas => ({ rows: Number(canvas.dataset.revealedRows), characters: Number(canvas.dataset.typedCharacters) }));
  expect(next.characters).toBeGreaterThan(first.characters);
  expect(next.rows).toBeGreaterThanOrEqual(first.rows);
  expect(next.rows).toBeLessThan(15);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(intro).toHaveAttribute("data-intro-state", "complete");
  expect(Number(await intro.getAttribute("data-revealed-rows"))).toBeGreaterThan(next.rows);
});

test("opening uses one file status bar and keeps it inside the viewport", async ({ page }) => {
  await open(page, "/");
  const status = page.locator(".intro-modes");
  await expect(status.locator(".mode-label")).toHaveCount(1);
  await expect(status.locator(".mode-label")).toHaveText("NORMAL");
  await expect(status.locator(".intro-file")).toHaveText("supa-agent/app.js");
  await expect(status.locator(".intro-position")).toHaveText(/\d+:\d+/);
  await expect(page.locator(".intro-command")).toBeVisible();
  const box = await status.boundingBox();
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(page.viewportSize().width);
  await page.locator(".theme-key").click();
  await expect(status).toBeVisible();
  await page.locator(".enter").click();
  await expect(page.locator("body")).toHaveAttribute("data-view", "projects");
  await expect(status).toBeHidden();
});

test("Supa backdrop preserves the approved pixel mask and stays behind readable code", async ({ page, request }) => {
  await open(page, "/");
  const logo=page.locator(".intro-mascot");
  await expect(logo).toBeVisible();
  const bounds=await logo.boundingBox(),viewport=page.viewportSize();
  expect(bounds.width).toBeLessThanOrEqual(220);expect(bounds.x).toBeGreaterThan(viewport.width/2);expect(bounds.y).toBeLessThan(150);
  expect(bounds.x+bounds.width).toBeLessThanOrEqual(viewport.width-20);
  const source=readFileSync(new URL("../../web/src/components/visuals.tsx", import.meta.url),"utf8");
  const original=source.slice(source.indexOf("export function PixelMark"),source.indexOf("const stages"));
  const paths=[...original.matchAll(/<path d="([^"]+)"/g)].map(match=>match[1]);
  expect(paths).toHaveLength(2);
  const comparison=await logo.evaluate(async (image, paths)=>{
    await image.decode();
    const canonical=new Image();canonical.src='/public/assets/supa-agent.svg';await canonical.decode();
    const results=[];
    for(const size of [24,48,96]){
      const actual=document.createElement("canvas"),expected=document.createElement("canvas");
      actual.width=actual.height=expected.width=expected.height=size;
      const a=actual.getContext("2d"),e=expected.getContext("2d");
      a.fillStyle=e.fillStyle="#0c0c0c";a.fillRect(0,0,size,size);e.fillRect(0,0,size,size);
      a.imageSmoothingEnabled=false;a.drawImage(canonical,0,0,size,size);
      e.scale(size/24,size/24);e.fillStyle="#71957d";e.fill(new Path2D(paths[0]));e.fillStyle="#0c0c0c";e.fill(new Path2D(paths[1]));
      const pixels=a.getImageData(0,0,size,size).data,reference=e.getImageData(0,0,size,size).data;
      results.push({size,identical:pixels.every((value,index)=>value===reference[index])});
    }
    const style=getComputedStyle(image),canvas=getComputedStyle(document.querySelector(".intro-canvas"));
    return {results,opacity:Number(style.opacity),filter:style.filter,belowCode:Number(style.zIndex)<Number(canvas.zIndex)};
  },paths);
  expect(comparison.results.every(result=>result.identical)).toBe(true);
  expect(comparison.opacity).toBeGreaterThan(0);
  expect(comparison.opacity).toBeLessThan(.75);
  expect(comparison.filter).toContain("blur(");
  expect(comparison.belowCode).toBe(true);
  for(const name of ["supa-agent.svg","supa-agent-icon.svg","supa-agent-header.svg","supa-agent-monochrome.svg","supa-agent-code.svg"]){
    const response=await request.get(`/public/assets/${name}`);
    expect(response.status()).toBe(200);
    expect(response.headers()["content-type"]).toContain("image/svg+xml");
  }
  await page.locator(".enter").click();
  await expect(logo).toBeHidden();
});

test("centered keyboard animates during typing and rests under reduced motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await open(page, "/");
  const intro=page.locator(".intro-canvas");
  await expect(intro).toHaveAttribute("data-hand-state", "typing");
  const bounds=(await intro.getAttribute("data-keyboard-bounds")).split(",").map(Number);
  expect(bounds[0]+bounds[2]/2).toBeCloseTo(page.viewportSize().width/2, 1);
  expect(bounds[2]).toBeGreaterThan(page.viewportSize().width<500?270:500);
  expect(bounds[1]+bounds[3]).toBeLessThan(page.viewportSize().height-25);
  const fingerprint=()=>intro.evaluate(canvas=>{
    const [x,y,w,h]=canvas.dataset.keyboardBounds.split(",").map(Number),scale=canvas.width/innerWidth;
    const pixels=canvas.getContext("2d").getImageData(Math.floor(x*scale),Math.floor(y*scale),Math.floor(w*scale),Math.floor(h*scale)).data;
    let hash=0;for(let i=0;i<pixels.length;i+=4)hash=(Math.imul(hash,31)+pixels[i])>>>0;
    return hash;
  });
  const first=await fingerprint();
  await expect.poll(fingerprint).not.toBe(first);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(intro).toHaveAttribute("data-hand-state", "idle");
  const resting=await fingerprint();
  await page.waitForTimeout(200);
  expect(await fingerprint()).toBe(resting);
});

test("Enter shows an opaque radial portal before navigating exactly once", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await open(page, "/");
  const historyLength = await page.evaluate(() => history.length);
  await page.locator(".enter").click();
  const portal = page.locator(".transition");
  await expect(portal).toHaveClass(/active/);
  const brand=page.locator(".portal-brand");
  await expect(brand).toBeVisible();
  await expect(brand).toHaveText("SUPA AGENT");
  await expect(brand.locator("img")).toHaveAttribute("src", "/public/assets/supa-agent-code.svg");
  const box=await brand.boundingBox(),viewport=page.viewportSize();
  expect(box.x+box.width/2).toBeCloseTo(viewport.width/2, 0);
  expect(box.y+box.height/2).toBeCloseTo(viewport.height/2, 0);
  expect(Math.hypot(box.width/2,box.height/2)).toBeLessThan(Math.min(viewport.width,viewport.height)*.29);
  await expect(page.locator("body")).toHaveAttribute("data-view", "intro");
  await expect(page).toHaveURL(/\/$/);
  const pixels = await portal.evaluate(canvas => {
    const ctx = canvas.getContext("2d");
    return { center: Array.from(ctx.getImageData(Math.floor(canvas.width/2), Math.floor(canvas.height/2), 1, 1).data), corner: Array.from(ctx.getImageData(0, 0, 1, 1).data) };
  });
  expect(pixels.center).toEqual([0, 0, 0, 255]);
  expect(pixels.corner[3]).toBe(255);
  await page.keyboard.press("Enter");
  await page.keyboard.press("Enter");
  await expect(page.locator("body")).toHaveAttribute("data-view", "projects");
  await expect(portal).not.toHaveClass(/active/);
  await expect(brand).toBeHidden();
  expect(await page.evaluate(() => history.length)).toBe(historyLength + 1);
  await page.goBack();
  await expect(page.locator("body")).toHaveAttribute("data-view", "intro");
});

test("reducing motion completes a portal and navigation changes cancel stale entry", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await open(page, "/");
  await page.locator(".enter").click();
  await expect(page.locator(".transition")).toHaveClass(/active/);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator("body")).toHaveAttribute("data-view", "projects");
  await expect(page.locator(".transition")).not.toHaveClass(/active/);
  await expect(page.locator(".portal-brand")).toBeHidden();
  await page.goBack();
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.locator(".enter").click();
  await expect(page.locator(".transition")).toHaveClass(/active/);
  // A popstate while the promise is pending must invalidate that entry attempt.
  await page.evaluate(() => window.dispatchEvent(new PopStateEvent("popstate")));
  await expect(page.locator(".transition")).not.toHaveClass(/active/);
  await expect(page.locator(".portal-brand")).toBeHidden();
  await page.waitForTimeout(2600);
  await expect(page.locator("body")).toHaveAttribute("data-view", "intro");
  await expect(page).toHaveURL(/\/$/);
});

test("changing projects cancels the previous terminal boot without restoring stale text", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/site/", { waitUntil: "domcontentloaded" });
  await expect(page.locator("html")).toHaveAttribute("data-ready", "true");
  await expect(page.locator(".buffer")).toHaveAttribute("aria-busy", "true");
  await page.keyboard.press("Escape");
  await expect(navigationFocus(page)).toBeFocused();
  if (page.viewportSize().width <= 700) await page.locator("button.tabline").click();
  await page.locator(".wheel").focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.locator('.wheel-item[aria-selected="true"]')).toHaveAttribute("data-index", "1");
  await expect(page.locator(".buffer")).toHaveAttribute("aria-busy", "true");
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator(".buffer")).toHaveAttribute("aria-busy", "false");
  await expect(page.locator(".buffer")).toContainText("Existing agents");
  await expect(page.locator(".buffer")).not.toContainText("Platform vision");
  await expect(page.locator(".buffer")).toContainText("saved records");
  await expect(page.locator(".entity-state b").nth(1)).toHaveText("idle");
});

test("mobile drawer toggles and Escape restores its trigger focus", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await open(page, "/site/");
  await expect(page.locator(".wheel-item").first()).toBeAttached();
  const toggle = page.locator("button.tabline");
  await expect(toggle).toBeVisible();
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(toggle).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("button.tabline")).toBeFocused();
});

test("project wheel responds to pointer scrolling and card selection", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await open(page, "/site/");
  await expect(page.locator(".wheel-item").first()).toBeAttached();
  const selected = () => page.locator('.wheel-item[aria-selected="true"]');
  const initial = await selected().getAttribute("data-index");
  await page.locator(".wheel").hover();
  await page.mouse.wheel(0, 100);
  await expect.poll(() => selected().getAttribute("data-index")).not.toBe(initial);
  const target = page.locator(`.wheel-item[data-index="${initial}"]`);
  await target.click();
  await expect(selected()).toHaveAttribute("data-index", initial);
});

test("Supa browser uses same-origin read APIs without paid or external requests", async ({ page }) => {
  const requests = [];
  const errors = [];
  page.on("request", request => requests.push(request.url()));
  page.on("pageerror", error => errors.push(error.message));
  await open(page, "/");
  await page.locator("a.enter").click();
  await expect(page.locator("body")).toHaveAttribute("data-view", "projects");
  await expect(page.locator(".bar")).toBeVisible();
  for (const url of requests) expect(["http://127.0.0.1:3201"]).toContain(new URL(url).origin);
  expect(requests.some(url => /(?:api\.openai|solana|analytics|plausible)/i.test(url))).toBe(false);
  expect(errors).toEqual([]);
});

test("responsive scenes fit all supported widths with reduced motion", async ({ page }) => {
  for (const width of [320, 390, 768, 1440]) {
    await page.setViewportSize({ width, height: width < 500 ? 844 : 1000 });
    for (const route of ["/", "/site/"]) {
      await open(page, route);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await expect(page.locator("html")).toHaveAttribute("data-motion", "still");
    }
  }
});

test("preview is read only and cannot expose project secrets or parent paths", async ({ request }) => {
  const health = await request.get("/healthz");
  expect(health.status()).toBe(200);
  expect(await health.json()).toMatchObject({ mode: "supa-bounded-offline", host: "127.0.0.1" });
  expect(health.headers()["content-security-policy"]).toContain("connect-src 'self'");
  expect(health.headers()["content-security-policy"]).toContain("frame-ancestors 'none'");
  for (const path of ["/.env", "/.env.eval", "/server.mjs", "/package.json", "/public/.env", "/%2e%2e/.env.eval", "/public/%2e%2e/%2e%2e/.env.eval", "/%2f%2e%2e%2f.env.eval"]) {
    const response = await request.get(path);
    expect([400, 404]).toContain(response.status());
    expect(await response.text()).not.toMatch(/sk-[A-Za-z0-9]|OPENAI_API_KEY|postgres(ql)?:\/\//);
  }
  expect((await request.post("/", { data: "mutation" })).status()).toBe(405);
});

test("code logo keeps the canonical silhouette and transparent facial cutouts", async ({ page, request }) => {
  await open(page,"/");
  const svg=await (await request.get('/public/assets/supa-agent-code.svg')).text();
  expect(svg).toContain('clip-rule="evenodd"');expect(svg).toContain('const');expect(svg).toContain('<text');expect(svg).toContain('<tspan');
  const solid=await (await request.get('/public/assets/supa-agent.svg')).text();
  expect(svg.match(/ d="([^"]+)"/)[1]).toBe(solid.match(/ d="([^"]+)"/)[1]);
  const raster=await page.locator('.intro-mascot').evaluate(async image=>{
    await image.decode();const canvas=document.createElement('canvas');canvas.width=canvas.height=240;
    const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0,240,240);
    const data=ctx.getImageData(0,0,240,240).data;
    const area=(x,y,w,h)=>{let pixels=0;for(let j=y;j<y+h;j++)for(let i=x;i<x+w;i++)if(data[(j*240+i)*4+3])pixels++;return pixels;};
    return {leftEye:area(60,90,40,50),rightEye:area(140,90,40,50),mouth:area(80,170,80,20),outside:area(0,0,240,20),ink:area(10,20,220,200),gaps:area(100,90,40,50)};
  });
  expect(raster.leftEye+raster.rightEye+raster.mouth+raster.outside).toBe(0);
  expect(raster.ink).toBeGreaterThan(2000);expect(raster.gaps).toBeLessThan(2000);
});


test("returning to the intro restarts its code and typing hands from the top", async ({ page }) => {
  await page.emulateMedia({reducedMotion:'no-preference'});await open(page,'/');
  const intro=page.locator('.intro-canvas');
  await expect.poll(async()=>Number(await intro.getAttribute('data-typed-characters'))).toBeGreaterThan(20);
  await page.locator('.enter').click();await expect(page.locator('body')).toHaveAttribute('data-view','projects');
  await page.goBack();await expect(page.locator('body')).toHaveAttribute('data-view','intro');
  expect(Number(await intro.getAttribute('data-revealed-rows'))).toBeLessThan(2);
  await expect(intro).toHaveAttribute('data-hand-state','typing');
  await page.locator('.enter').click();await expect(page.locator('body')).toHaveAttribute('data-view','projects');
  await page.locator('.brand').click();await expect(page.locator('body')).toHaveAttribute('data-view','intro');
  expect(Number(await intro.getAttribute('data-revealed-rows'))).toBeLessThan(2);
  await expect(intro).toHaveAttribute('data-hand-state','typing');
  await page.emulateMedia({reducedMotion:'reduce'});await expect(intro).toHaveAttribute('data-intro-state','complete');
});

test('mascot draws a full-viewport pointer tether without intercepting input', async ({ page }) => {
  await open(page, '/site/#home');
  const tether=page.locator('.pointer-tether');
  for(const reducedMotion of ['reduce','no-preference']){
    await page.emulateMedia({reducedMotion});
    const {width,height}=page.viewportSize(),x=Math.round(width*.55),y=Math.round(height*.7);
    await page.mouse.move(x,y);
    await expect(tether).toHaveAttribute('data-active','true');
    const result=await tether.evaluate(canvas=>{
      const [sx,sy,x,y]=canvas.dataset.endpoints.split(',').map(Number),box=document.querySelector('.entity').getBoundingClientRect();
      const scale=canvas.width/innerWidth,ctx=canvas.getContext('2d');let painted=0;
      // Check the middle of the long line, away from either endpoint.
      for(let t=.35;t<.65;t+=.005){const pixel=ctx.getImageData(Math.round((sx+(x-sx)*t)*scale),Math.round((sy+(y-sy)*t)*scale),1,1).data;if(pixel[3])painted++;}
      return {sx,sy,x,y,anchorX:box.left+box.width*.5,anchorY:box.top+box.height*.4,painted,pointerEvents:getComputedStyle(canvas).pointerEvents,hit:document.elementFromPoint(x,y)?.className};
    });
    expect(result.x).toBeCloseTo(x);expect(result.y).toBeCloseTo(y);expect(result.sx).toBeCloseTo(result.anchorX);expect(result.sy).toBeCloseTo(result.anchorY);
    expect(result.painted).toBeGreaterThan(5);expect(result.pointerEvents).toBe('none');expect(result.hit).not.toBe('pointer-tether');
  }
  await page.evaluate(()=>document.documentElement.dispatchEvent(new PointerEvent('pointerleave')));
  await expect(tether).toHaveAttribute('data-active','false');
  await page.evaluate(()=>window.dispatchEvent(new PointerEvent('pointermove',{clientX:100,clientY:200,pointerType:'touch'})));
  await expect(tether).toHaveAttribute('data-active','false');
});

test('decorative branches flow, pause and respect reduced motion and visibility', async ({ page }) => {
  await page.emulateMedia({reducedMotion:'no-preference'});await open(page,'/site/#home');
  const diagram=page.locator('.blueprint');
  const pixels=()=>diagram.evaluate(canvas=>{const data=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;let hash=0;for(let i=0;i<data.length;i+=4)hash=(Math.imul(hash,31)+data[i]+data[i+3])|0;return hash;});
  await expect(diagram).toHaveAttribute('data-motion-state','running');
  const first=await pixels();await page.waitForTimeout(250);expect(await pixels()).not.toBe(first);
  await page.locator('.command-toggle').click();const control=page.getByRole('button',{name:'Animate decorative diagram'});
  await control.click();await expect(diagram).toHaveAttribute('data-motion-state','paused');await expect(control).toHaveText('Play diagram');
  const paused=await pixels();await page.waitForTimeout(180);expect(await pixels()).toBe(paused);
  await control.click();await expect(diagram).toHaveAttribute('data-motion-state','running');await expect(control).toHaveText('Pause diagram');
  await diagram.evaluate(canvas=>canvas.style.visibility='hidden');
  // IntersectionObserver follows geometry/display, rather than paint visibility.
  await diagram.evaluate(canvas=>canvas.style.display='none');await expect(diagram).toHaveAttribute('data-motion-state','offscreen');
  const stopped=await diagram.getAttribute('data-motion-phase');await page.waitForTimeout(180);expect(await diagram.getAttribute('data-motion-phase')).toBe(stopped);
  await diagram.evaluate(canvas=>{canvas.style.removeProperty('display');canvas.style.removeProperty('visibility');});await expect(diagram).toHaveAttribute('data-motion-state','running');
  await page.emulateMedia({reducedMotion:'reduce'});await expect(diagram).toHaveAttribute('data-motion-state','reduced');await expect(control).toBeDisabled();
  const still=await pixels();await page.waitForTimeout(180);expect(await pixels()).toBe(still);
  await expect(diagram).toHaveAttribute('aria-label',/not live agent activity/);
});

test('intro pointer follows the mouse from the code mascot and survives entry', async ({ page }) => {
  await open(page,'/');const tether=page.locator('.pointer-tether');
  for(const reducedMotion of ['reduce','no-preference']){
    await page.emulateMedia({reducedMotion});await page.mouse.move(280,200);await page.mouse.move(300,220);
    await expect(tether).toHaveAttribute('data-active','true');
    await expect.poll(async()=>{const p=(await tether.getAttribute('data-endpoints')).split(',').map(Number);return Math.abs(p[2]-300)+Math.abs(p[3]-220);}).toBeLessThan(.01);
    const anchor=await page.locator('.intro-mascot').boundingBox();
    const [x,y]= (await tether.getAttribute('data-endpoints')).split(',').map(Number);
    expect(x).toBeCloseTo(anchor.x+anchor.width*.5);expect(y).toBeCloseTo(anchor.y+anchor.height*.4);
    expect(await tether.evaluate(canvas=>canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data.some((value,i)=>i%4===3&&value>0))).toBe(true);
  }
  await page.evaluate(()=>{
    window.tetherHiddenDuringPortal=false;
    const observer=new MutationObserver(()=>{if(document.querySelector('.transition').classList.contains('active')&&document.querySelector('.pointer-tether').dataset.active==='false'){window.tetherHiddenDuringPortal=true;observer.disconnect();}});
    observer.observe(document.querySelector('.pointer-tether'),{attributes:true,attributeFilter:['data-active']});
  });
  await page.locator('.enter').click();await expect(page.locator('body')).toHaveAttribute('data-view','projects');
  expect(await page.evaluate(()=>window.tetherHiddenDuringPortal)).toBe(true);await page.mouse.move(310,230);await expect(tether).toHaveAttribute('data-active','true');
  await page.locator('.brand').click();await expect(page.locator('body')).toHaveAttribute('data-view','intro');await page.mouse.move(320,240);await expect(tether).toHaveAttribute('data-active','true');
  await page.evaluate(()=>window.dispatchEvent(new PointerEvent('pointermove',{clientX:100,clientY:200,pointerType:'touch'})));await expect(tether).toHaveAttribute('data-active','false');
});

test('hands switch between mouse and typing while a second tether follows screen text', async ({page})=>{
  await page.emulateMedia({reducedMotion:'no-preference'});await open(page,'/');
  const intro=page.locator('.intro-canvas'),tether=page.locator('.pointer-tether');
  await page.mouse.move(150,300);await expect(intro).toHaveAttribute('data-right-hand','mouse');await expect(intro).toHaveAttribute('data-left-hand','keyboard');
  await expect(tether).toHaveAttribute('data-typing-active','true');await expect(tether).toHaveAttribute('data-active','true');
  const keyboardBounds=await intro.getAttribute('data-keyboard-bounds');
  await expect(intro).toHaveAttribute('data-right-hand','keyboard');expect(await intro.getAttribute('data-keyboard-bounds')).toBe(keyboardBounds);await expect(intro).toHaveAttribute('data-hand-state','typing');
  await open(page,'/site/#about');const hands=page.locator('.workspace-hands');
  await expect(hands).toBeVisible();await expect(hands).toHaveAttribute('data-hand-state','typing');
  await expect(tether).toHaveAttribute('data-typing-active','true');
  const target=await tether.evaluate(canvas=>canvas.dataset.typingEndpoints.split(',').map(Number));const buffer=await page.locator('.buffer').boundingBox();
  expect(target[2]).toBeGreaterThanOrEqual(buffer.x);expect(target[2]).toBeLessThanOrEqual(buffer.x+buffer.width);expect(target[3]).toBeGreaterThan(buffer.y);expect(target[3]).toBeLessThan(buffer.y+buffer.height);
  await page.mouse.move(170,310);await expect(hands).toHaveAttribute('data-right-hand','mouse');await expect(hands).toHaveAttribute('data-right-hand','keyboard');
  await page.evaluate(()=>window.dispatchEvent(new PointerEvent('pointermove',{clientX:180,clientY:320,pointerType:'touch'})));await expect(hands).toHaveAttribute('data-right-hand','keyboard');
  await page.emulateMedia({reducedMotion:'reduce'});await expect(hands).toHaveAttribute('data-hand-state','idle');
  await expect(tether).toHaveAttribute('data-typing-active','false');
  expect(await hands.evaluate(canvas=>getComputedStyle(canvas).pointerEvents)).toBe('none');
});

test('typing tether disappears on completion while mouse tracking remains available',async({page})=>{
  // Short public source fixture lets the intro finish naturally, without skipping time.
  await page.route('**/app.js',route=>route.request().resourceType()==='fetch'?route.fulfill({contentType:'text/javascript',body:';'}):route.continue());
  await page.emulateMedia({reducedMotion:'no-preference'});await open(page,'/');
  const tether=page.locator('.pointer-tether');
  await expect(tether).toHaveAttribute('data-typing-active','true');
  await expect(page.locator('.intro-canvas')).toHaveAttribute('data-intro-state','complete',{timeout:25000});
  await expect(tether).toHaveAttribute('data-typing-active','false');
  await page.mouse.move(220,190);await expect(tether).toHaveAttribute('data-active','true');
  await open(page,'/site/#about');
  await expect(tether).toHaveAttribute('data-typing-active','true');
  await page.locator('.command-toggle').click();await page.locator('.skip-typing').click();
  await expect(tether).toHaveAttribute('data-typing-active','false');
  await page.mouse.move(230,200);await expect(tether).toHaveAttribute('data-active','true');
  await page.locator('.brand').click();
  await expect(tether).toHaveAttribute('data-typing-active','true');
  await page.emulateMedia({reducedMotion:'reduce'});
  await expect(tether).toHaveAttribute('data-typing-active','false');
});

test('wheel uses distinct section schematics and full planned labels',async({page,request})=>{
  await open(page,'/site/#about');
  if(page.viewportSize().width<=700)await page.locator('.tabline').click();
  const source=await request.get('/public/assets/navigation-previews.svg');expect(source.status()).toBe(200);expect(source.headers()['content-type']).toContain('image/svg+xml');
  const previews=page.locator('.wheel-preview use');await expect(previews).toHaveCount(9);
  const references=await previews.evaluateAll(nodes=>nodes.map(node=>node.getAttribute('href')));expect(new Set(references).size).toBe(9);
  expect(references.every(value=>value.startsWith('/public/assets/navigation-previews.svg#'))).toBe(true);
  await expect(page.locator('.wheel-item[data-index="2"] .wheel-name')).toHaveText('Intelligence');
  await expect(page.locator('.wheel-item[data-index="7"] .wheel-name')).toHaveText('Token Launch');
  await expect(page.locator('.wheel-planned')).toHaveCount(3);
  for(const index of [6,7,8])await expect(page.locator(`.wheel-item[data-index="${index}"] .wheel-planned`)).toHaveText('planned');
  await expect.poll(()=>page.locator('.wheel-item[data-index="9"] use').evaluate(use=>use.getBBox().width)).toBeGreaterThan(0);
  await expect(page.locator('.wheel-item[data-index="9"]')).toHaveAttribute('aria-selected','true');
  if(page.viewportSize().width<=700){const selected=await page.locator('.wheel-item[data-index="9"]').boundingBox();expect(Math.abs(selected.x+selected.width/2-page.viewportSize().width/2)).toBeLessThan(5);}
  const name=page.locator('.wheel-item[data-index="9"] .wheel-name');expect(await name.evaluate(el=>el.scrollWidth<=el.clientWidth)).toBe(true);
});

test('navigation fibers keep moving while cards stay still and can be paused',async({page})=>{
  await page.emulateMedia({reducedMotion:'no-preference'});await open(page,'/site/#home');
  if(page.viewportSize().width<=700)await page.locator('.tabline').click();
  const canvas=page.locator('.wheel-ink'),selected=page.locator('.wheel-item[aria-selected=true]');
  const pixels=()=>canvas.evaluate(el=>{const data=el.getContext('2d').getImageData(0,0,el.width,el.height).data;let hash=0;for(let i=0;i<data.length;i+=4)hash=(Math.imul(hash,31)+data[i]+data[i+3])|0;return hash;});
  await expect(canvas).toHaveAttribute('data-motion-state','running');
  const first=await pixels(),transform=await selected.getAttribute('style');await expect.poll(pixels).not.toBe(first);expect(await selected.getAttribute('style')).toBe(transform);
  await page.keyboard.press(':');const control=page.getByRole('button',{name:'Animate navigation branches'});await control.click();await expect(canvas).toHaveAttribute('data-motion-state','paused');await expect(control).toHaveText('Play branches');
  const frozen=await pixels();await page.waitForTimeout(200);expect(await pixels()).toBe(frozen);
  await control.click();await expect(canvas).toHaveAttribute('data-motion-state','running');
  await page.locator('.wheel').evaluate(el=>el.style.display='none');await expect(canvas).toHaveAttribute('data-motion-state','offscreen');
  const phase=await canvas.getAttribute('data-motion-phase');await page.waitForTimeout(200);expect(await canvas.getAttribute('data-motion-phase')).toBe(phase);
  await page.locator('.wheel').evaluate(el=>el.style.removeProperty('display'));await expect(canvas).toHaveAttribute('data-motion-state','running');
  await page.emulateMedia({reducedMotion:'reduce'});await expect(canvas).toHaveAttribute('data-motion-state','reduced');await expect(control).toBeDisabled();
  const still=await pixels();await page.waitForTimeout(200);expect(await pixels()).toBe(still);
});

test('every workspace view anchors the keyboard below content and enlarges architecture',async({page})=>{
  for(const section of ['home','agents','intelligence','memory','economy','social','trading','token-launch','factory','about']){
    await open(page,`/site/#${section}`);
    const keyboard=await page.locator('.workspace-hands').boundingBox(),bar=await page.locator('.bar').boundingBox(),buffer=await page.locator('.buffer').boundingBox();
    expect(keyboard.y+keyboard.height).toBeCloseTo(page.viewportSize().height,0);expect(bar.y+bar.height).toBeLessThanOrEqual(keyboard.y+1);
    expect(buffer.height).toBeGreaterThan(60);expect(buffer.y+buffer.height).toBeLessThanOrEqual(bar.y+1);
    if(['home','about'].includes(section))expect((await page.locator('.blueprint').boundingBox()).height).toBeGreaterThanOrEqual(190);
  }
  await page.setViewportSize({width:360,height:640});await open(page,'/site/#home');
  expect((await page.locator('.blueprint').boundingBox()).height).toBeGreaterThanOrEqual(160);
  const keyboard=await page.locator('.workspace-hands').boundingBox();expect(keyboard.y+keyboard.height).toBeCloseTo(640,0);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
});
