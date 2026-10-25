// Use an existing Playwright installation; this script never installs browsers.
// ZDICT_PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs
// ZDICT_CHROMIUM=/absolute/path/to/chrome node tests/smoke.mjs
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const { chromium } = await import(process.env.ZDICT_PLAYWRIGHT_MODULE ?? 'playwright');
const browser = await chromium.launch({ headless: true, ...(process.env.ZDICT_CHROMIUM ? { executablePath: process.env.ZDICT_CHROMIUM } : {}) });
const page = await browser.newPage({ viewport: { width: 1100, height: 850 }, colorScheme: 'light' });
const errors = [];
page.on('pageerror', error => errors.push(error.message));
const input = page.locator('input[role="combobox"]');
const pane = page.locator('.entry-pane');
const results = page.locator('#lookup-results');
const screenshots = fileURLToPath(new URL('../plan/m3/', import.meta.url));
await mkdir(screenshots, { recursive: true });
async function entry(word) {
  await page.waitForFunction(word => document.querySelector('.dx-headword')?.textContent === word, word);
  await page.waitForFunction(() => window.__zdictPerf.entry.length > 0);
}
async function query(text) {
  await input.fill(text);
  await page.waitForFunction(() => document.querySelector('#lookup-results')?.getAttribute('aria-busy') === 'false');
}
async function settings() { await page.getByRole('button', { name: '设置', exact: true }).click(); }
async function closeSettings() { await page.getByRole('button', { name: '关闭设置' }).click(); }
async function chooseTheme(theme) { await page.getByLabel('外观', { exact: true }).selectOption(theme); }

try {
  await page.goto(process.env.ZDICT_URL ?? 'http://127.0.0.1:1420/');
  await page.getByText('小小造词集', { exact: true }).waitFor();
  assert.equal(await input.evaluate(el => document.activeElement === el), true, 'search autofocus');

  await query('mo');
  assert.equal(await results.getByRole('option').count(), 2);
  await input.press('ArrowDown'); await input.press('ArrowDown'); await input.press('Enter');
  await entry('moonjar');
  await page.locator('[data-anchor="2"].anchor-highlight').waitFor();
  await page.waitForFunction(() => document.querySelector('img[data-fallback]')?.naturalWidth > 0);
  await page.evaluate(() => {
    window.__audioAttempts = [];
    window.__audioPlayed = false;
    const original = HTMLMediaElement.prototype.play;
    HTMLMediaElement.prototype.play = function () {
      window.__audioAttempts.push(this.src);
      this.addEventListener('playing', () => { window.__audioPlayed = true; }, { once: true });
      return original.call(this);
    };
  });
  await page.getByRole('button', { name: '播放示例声音' }).click();
  await page.waitForFunction(() => window.__audioAttempts.length === 2 && window.__audioPlayed);

  await page.locator('a.dx-link').filter({ hasText: /^hushleaf$/ }).click();
  await entry('hushleaf');
  await page.keyboard.press('Alt+ArrowLeft'); await entry('moonjar');
  await page.keyboard.press('Alt+ArrowRight'); await entry('hushleaf');
  await page.getByRole('button', { name: '后退', exact: true }).click(); await entry('moonjar');
  await page.locator('a.dx-link').filter({ hasText: '查看词族' }).click();
  await page.locator('.dx-word_family.anchor-highlight').waitFor();
  assert.equal(await page.getByRole('button', { name: '前进', exact: true }).isDisabled(), true, 'new visit truncates forward history');
  await page.locator('a.dx-link').filter({ hasText: /^fernlight$/ }).click(); await entry('fernlight');
  assert.equal(await input.inputValue(), 'fernlight', 'lookup link replaces the query');

  await query('monjar');
  await page.getByText('你是不是要找：', { exact: true }).waitFor();
  await input.press('Enter'); await entry('moonjar');
  await input.press('Escape');
  assert.equal(await input.inputValue(), '');
  assert.ok((await results.innerText()).includes('moonjar'), 'recent entries shown on clear');

  await page.keyboard.press('Control+Shift+F');
  await query('wish');
  await page.getByRole('button', { name: '释义', exact: true }).click();
  await page.getByText('没有找到匹配的词。', { exact: true }).waitFor();
  await page.getByRole('button', { name: '例句', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('#lookup-results [role="option"]') !== null);
  assert.match(await results.innerText(), /moonjar\s+en\s+词条内 · #1/);
  await input.press('Enter'); await entry('moonjar');
  await page.locator('[data-anchor="1"].anchor-highlight').waitFor();
  await page.keyboard.press('Control+Shift+F');

  await settings();
  await page.getByLabel('隐藏中文', { exact: true }).check();
  await page.waitForFunction(() => !document.querySelector('.dx-entry [lang="zh-CN"]'));
  await page.getByLabel('隐藏音标', { exact: true }).check();
  await page.waitForFunction(() => !document.querySelector('.dx-pronunciation'));
  await page.getByLabel('隐藏例句', { exact: true }).check();
  await page.waitForFunction(() => !document.querySelector('.dx-example'));
  await page.getByLabel('字号', { exact: false }).fill('1.3');
  await page.waitForFunction(() => document.querySelector('.entry-pane')?.style.getPropertyValue('--entry-font-scale') === '1.3');
  await page.getByLabel('隐藏中文', { exact: true }).uncheck();
  await page.waitForFunction(() => document.querySelector('.dx-entry [lang="zh-CN"]'));
  await page.getByLabel('隐藏音标', { exact: true }).uncheck();
  await page.locator('.dx-pronunciation').waitFor();
  await page.getByLabel('隐藏例句', { exact: true }).uncheck();
  await page.locator('.dx-example').waitFor();
  await page.getByLabel('字号', { exact: false }).fill('1');
  await chooseTheme('light');
  await closeSettings();
  await query('moon');
  await page.keyboard.press('Control+Shift+D');
  await page.getByRole('complementary', { name: '延迟统计' }).waitFor();
  const perf = await page.evaluate(() => window.__zdictPerf);
  assert.ok(perf.suggest.length > 0 && perf.entry.length > 0, 'latency buffers populated');
  assert.equal(await page.locator('#zdict-entry-theme').count(), 1, 'one injected theme');
  await page.getByRole('button', { name: '关闭延迟统计' }).click();
  await pane.evaluate(el => { el.scrollTop = 0; });
  await input.evaluate(el => el.blur());
  await page.screenshot({ path: `${screenshots}/desktop-light.png` });

  await settings(); await chooseTheme('dark'); await closeSettings();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  await input.evaluate(el => el.blur());
  await page.screenshot({ path: `${screenshots}/desktop-dark.png` });
  await page.reload();
  await page.getByText('小小造词集', { exact: true }).waitFor();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'theme persists');
  assert.ok((await results.innerText()).includes('moonjar'), 'recent history persists');
  await input.press('Enter'); await entry('moonjar');

  await page.setViewportSize({ width: 420, height: 800 });
  await page.waitForFunction(() => document.documentElement.scrollWidth <= 420);
  const listBox = await page.locator('.results-pane').boundingBox();
  const entryBox = await pane.boundingBox();
  assert.ok(listBox.y + listBox.height <= entryBox.y + 1, 'mini window stacks list above entry');
  await settings();
  await pane.evaluate(el => { el.scrollTop = 130; });
  const before = await pane.evaluate(el => el.scrollTop);
  await page.getByLabel('隐藏例句', { exact: true }).check();
  await page.waitForFunction(() => !document.querySelector('.dx-example'));
  await page.waitForFunction(() => window.__zdictPerf.entry.length >= 2);
  const after = await pane.evaluate(el => el.scrollTop);
  assert.ok(Math.abs(after - before) < 3, `settings preserves entry scroll: ${before} → ${after}`);
  await page.getByLabel('字号', { exact: false }).fill('1.4');
  await page.waitForFunction(() => document.querySelector('.entry-pane')?.style.getPropertyValue('--entry-font-scale') === '1.4');
  await chooseTheme('system');
  assert.equal(await page.locator('html').getAttribute('data-theme'), null, 'system removes explicit theme');
  await closeSettings();
  await page.reload();
  await settings();
  assert.equal(await page.getByLabel('隐藏例句', { exact: true }).isChecked(), true, 'settings persist');
  assert.equal(await page.getByLabel('字号', { exact: false }).inputValue(), '1.4');
  await closeSettings();
  await input.press('Enter'); await entry('moonjar');
  await page.screenshot({ path: `${screenshots}/mini-420.png` });

  await page.locator('.dx-headword').click();
  await page.keyboard.press('/');
  assert.equal(await input.evaluate(el => document.activeElement === el), true, 'slash focuses search');
  await page.keyboard.press('Control+l');
  assert.equal(await input.evaluate(el => document.activeElement === el), true, 'Ctrl+L focuses search');
  assert.deepEqual(errors, [], 'no uncaught browser errors');
  console.log(JSON.stringify({ status: 'passed', checks: 'typing, keyboard selection, anchors, images, audio, links, history, correction, full-text filters, settings, scroll, themes, persistence, performance, 420px layout', samples: { suggest: perf.suggest.length, entry: perf.entry.length }, screenshots }, null, 2));
} finally { await browser.close(); }
