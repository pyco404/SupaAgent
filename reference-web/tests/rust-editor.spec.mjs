import { test, expect } from '../../web/node_modules/@playwright/test/index.mjs';
import { readFileSync } from 'node:fs';
const source = readFileSync(new URL('../../crates/runtime/src/lib.rs', import.meta.url), 'utf8').split('\n');

async function open(page) {
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-ready', 'true');
}

test('displayed Rust excerpt exactly matches the complete real runtime function', async ({ page }) => {
  await open(page);
  const excerpt = await page.evaluate(async () => (await import('/scene.js')).RUNTIME_EXCERPT);
  expect(excerpt.file).toBe('crates/runtime/src/lib.rs');
  expect(excerpt.title).toContain('excerpt');
  const lines = excerpt.code.split('\n');
  expect(lines).toEqual(source.slice(excerpt.firstLine - 1, excerpt.firstLine - 1 + lines.length).map(line => line.slice(4)));
  expect(lines[0]).toBe('async fn cycle_inner(');
  expect(lines.at(-1)).toBe('}');
  await expect(page.locator('.intro-file')).toHaveText(excerpt.file);
  await expect(page.locator('.intro-canvas')).toHaveAttribute('data-source-file', excerpt.file);
  await expect(page.locator('.intro-position')).toHaveText('132:2');
});

test('Rust lexer preserves tokens and wrapping never discards source', async ({ page }) => {
  await open(page);
  const result = await page.evaluate(async () => {
    const { rustTokens, rustEditorRows, RUNTIME_EXCERPT } = await import('/scene.js');
    const sample = 'async fn run(&self) -> Result<()> { tracing::info!("await 180"); } // let 7';
    const tokens = rustTokens(sample);
    const lines = RUNTIME_EXCERPT.code.split('\n');
    return { sample, tokens, preserved: [18, 44, 100].every(limit => {
      const rows = rustEditorRows(lines, limit);
      return lines.every((line, i) => rows.filter(row => row.sourceLine === RUNTIME_EXCERPT.firstLine + i).map(row => row.text).join('') === line) && rows.every(row => row.text.length <= limit);
    }) };
  });
  expect(result.tokens.map(token => token.text).join('')).toBe(result.sample);
  expect(result.tokens).toContainEqual({ text: 'async', kind: 'keyword' });
  expect(result.tokens).toContainEqual({ text: 'fn', kind: 'keyword' });
  expect(result.tokens).toContainEqual({ text: 'info!', kind: 'macro' });
  expect(result.tokens).toContainEqual({ text: '"await 180"', kind: 'string' });
  expect(result.tokens).toContainEqual({ text: '// let 7', kind: 'comment' });
  expect(result.preserved).toBe(true);
});

test('Rust types progressively with cursor and hands, then becomes still with reduced motion', async ({ page }) => {
  const errors = [], fetched = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { if (request.resourceType() === 'fetch') fetched.push(request.url()); });
  await page.addInitScript(() => {
    window.rustPaint = [];
    const draw = CanvasRenderingContext2D.prototype.fillText;
    CanvasRenderingContext2D.prototype.fillText = function(text, ...args) {
      if (this.canvas.classList.contains('intro-canvas') && window.rustPaint.length < 1500) window.rustPaint.push({ text, color: this.fillStyle });
      return draw.call(this, text, ...args);
    };
  });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await open(page);
  const intro = page.locator('.intro-canvas');
  await expect(intro).toHaveAttribute('data-language', 'rust');
  await expect(page.locator('.mode-label')).toHaveText('INSERT');
  await expect(intro).toHaveAttribute('data-hand-state', 'typing');
  const first = Number(await intro.getAttribute('data-typed-characters'));
  await expect.poll(async () => Number(await intro.getAttribute('data-typed-characters'))).toBeGreaterThan(first + 12);
  await expect(page.locator('.pointer-tether')).toHaveAttribute('data-typing-active', 'true');
  const paint = await page.evaluate(() => window.rustPaint);
  expect(paint.some(token => token.text === 'async' && token.color === '#71957d')).toBe(true);
  await expect(page.locator('.intro-position')).toHaveText(/^10[6-9]:\d+$/);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(intro).toHaveAttribute('data-intro-state', 'complete');
  await expect(page.locator('.mode-label')).toHaveText('NORMAL');
  await expect(intro).toHaveAttribute('data-hand-state', 'idle');
  await expect(page.locator('.pointer-tether')).toHaveAttribute('data-typing-active', 'false');
  expect(fetched.some(url => url.endsWith('/app.js'))).toBe(false);
  expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)).toBe(false);
  expect(errors).toEqual([]);
});
