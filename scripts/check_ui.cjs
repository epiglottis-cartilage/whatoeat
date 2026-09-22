#!/usr/bin/env node
// Static SSR/WebView layout checks. This does not execute Dioxus event handlers.
// Build first: cargo build --features preview --bin preview
// Run: PLAYWRIGHT_MODULE=/path/to/playwright node scripts/check_ui.cjs
// Optional: PREVIEW_BIN, UI_CHECK_OUTPUT, UI_CHECK_BROWSERS=chromium,webkit,
// CHROMIUM_PATH (otherwise use system Chromium when present, then Playwright).
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { pathToFileURL } = require('node:url');
const { values: options } = require('node:util').parseArgs({ options: {
  playwright: { type: 'string' }, themes: { type: 'string' },
  output: { type: 'string' }, browsers: { type: 'string' },
} });
const playwright = require(options.playwright || process.env.PLAYWRIGHT_MODULE || 'playwright');

const root = path.resolve(__dirname, '..');
const binary = path.resolve(process.env.PREVIEW_BIN || path.join(root, 'target/debug/preview'));
const output = path.resolve(options.output || process.env.UI_CHECK_OUTPUT || path.join(os.tmpdir(), 'whatoeat-ui-check'));
const browserNames = (options.browsers || process.env.UI_CHECK_BROWSERS || 'chromium').split(',');
const oldThemes = ['poster', 'rhodes', 'rhine', 'penguin', 'kazimierz'];
const interfaces = ['control', 'reclamation', 'expedition', 'automata', 'phantom', 'island', 'terminal', 'strand', 'frontline', 'marathon'];
const themes = [...oldThemes, ...interfaces];
const selectedThemes = (options.themes || process.env.UI_CHECK_THEMES)?.split(',') || themes;
for (const theme of selectedThemes) assert.ok(themes.includes(theme), `Unknown theme: ${theme}`);
const widths = [360, 390, 760, 1060];
const height = 840;
const cases = new Map();
const report = {
  started_at: new Date().toISOString(),
  preview_binary: binary,
  scope: 'SSR static DOM and browser layout; no live Dioxus events or native device execution',
  cases: [],
  screenshots: [],
};

function add(theme, screen, width, state = 'ready', toast = '') {
  if (!selectedThemes.includes(theme)) return;
  const key = [theme, screen, width, state, toast || 'none'].join('-');
  cases.set(key, { key, theme, screen, width, state, toast });
}

// Old palettes retain a narrow/wide smoke check. New interfaces cover every page
// at the four design breakpoints, then each distinct recommendation state.
for (const theme of oldThemes) {
  for (const width of [360, 1060]) add(theme, 'eat', width);
}
for (const theme of interfaces) {
  for (const screen of ['eat', 'history', 'foods', 'backup']) {
    for (const width of widths) add(theme, screen, width);
  }
  for (const state of ['idle', 'empty', 'exhausted', 'accepted', 'stale', 'loading', 'long', 'busy']) {
    add(theme, 'eat', 360, state);
  }
  add(theme, 'history', 390, 'long');
  add(theme, 'foods', 390, 'long');
  add(theme, 'foods', 390, 'ready', '1');
  add(theme, 'eat', 390, 'ready', 'error');
}

fs.mkdirSync(output, { recursive: true });
assert.ok(fs.existsSync(binary), `Build the preview executable first: ${binary}`);

const previews = new Map();
function previewFile(test) {
  const key = [test.theme, test.screen, test.state, test.toast || 'none'].join('-');
  if (previews.has(key)) return previews.get(key);
  const result = spawnSync(binary, [], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 8 * 1024 * 1024,
    env: {
      ...process.env,
      WHATOEAT_PREVIEW_THEME: test.theme,
      WHATOEAT_PREVIEW_SCREEN: test.screen,
      WHATOEAT_PREVIEW_STATE: test.state,
      WHATOEAT_PREVIEW_TOAST: test.toast,
    },
  });
  assert.equal(result.error, undefined, `Preview execution failed: ${result.error}`);
  assert.equal(result.status, 0, `Preview failed: ${result.stderr}`);
  assert.match(result.stdout, /<!doctype html>/i);
  const file = path.join(output, `${key}.html`);
  fs.writeFileSync(file, result.stdout);
  previews.set(key, file);
  return file;
}

async function snapshot(page) {
  return page.evaluate(() => {
    const rect = (selector) => {
      const element = document.querySelector(selector);
      if (!element) return null;
      const r = element.getBoundingClientRect();
      return { top: r.top, bottom: r.bottom, left: r.left, right: r.right, height: r.height };
    };
    return {
      nav: rect('.nav'),
      footer: rect('.footer'),
      main: rect('.main'),
      picker: rect('.theme-picker'),
      wordmark: rect('.wordmark'),
      documentWidth: document.documentElement.scrollWidth,
      viewportWidth: innerWidth,
      viewportHeight: innerHeight,
      scrollY,
      navPosition: getComputedStyle(document.querySelector('.nav')).position,
    };
  });
}

async function check(page, test, browserName) {
  await page.setViewportSize({ width: test.width, height });
  await page.goto(pathToFileURL(previewFile(test)).href);
  await page.evaluate(() => document.fonts.ready);
  assert.equal(await page.locator('.theme-root').getAttribute('data-theme'), test.theme);
  assert.equal(await page.locator('#theme-select').evaluate((e) => e.tagName), 'SELECT');
  assert.equal(await page.locator('#theme-select').inputValue(), test.theme);
  assert.equal(await page.locator('#theme-select option').count(), themes.length);
  assert.equal(await page.locator('.nav-item').count(), 4);
  assert.equal(await page.locator('.nav-item[aria-current="page"]').count(), 1);

  const top = await snapshot(page);
  assert.equal(top.documentWidth, test.width, 'horizontal overflow');
  assert.ok(top.picker.left >= 0 && top.picker.right <= test.width + 1, 'theme dropdown is clipped');
  const headerOverlaps = top.wordmark.left < top.picker.right
    && top.wordmark.right > top.picker.left
    && top.wordmark.top < top.picker.bottom
    && top.wordmark.bottom > top.picker.top;
  assert.ok(!headerOverlaps, 'wordmark and theme dropdown overlap');

  if (interfaces.includes(test.theme) && test.width <= 760 && test.screen === 'eat' && test.state !== 'loading') {
    const fill = await page.locator('.eat-console').evaluate((element) => ({
      panel: element.getBoundingClientRect().width,
      layout: element.closest('.eat-layout').getBoundingClientRect().width,
    }));
    assert.ok(fill.panel >= fill.layout * .95, 'mobile candidate panel leaves unused horizontal space');
  }

  if (test.screen === 'eat' && ['ready', 'long', 'busy'].includes(test.state)) {
    assert.equal(await page.locator('.decision').count(), 2);
    assert.equal(await page.locator('.food-name').count(), 1, 'only one candidate must be presented');
    const clipped = await page.locator('.food-name, .decision strong, .decision span').evaluateAll((elements) => (
      elements.map((el) => {
        const range = document.createRange();
        range.selectNodeContents(el);
        const rect = range.getBoundingClientRect();
        return { text: el.textContent, left: rect.left, right: rect.right };
      }).filter((rect) => rect.left < -1 || rect.right > innerWidth + 1)
    ));
    assert.deepEqual(clipped, [], 'candidate or decision text extends beyond viewport');
    if (test.state === 'busy') {
      assert.equal(await page.locator('.decision:disabled').count(), 2);
    }
  }

  if (['foods', 'history'].includes(test.screen)) {
    const controls = await page.locator('.main input, .main select, .main button').evaluateAll((elements) => (
      elements.filter((element) => element.getClientRects().length).map((element) => {
        const r = element.getBoundingClientRect();
        const owner = element.closest('.food-row, .history-row, .meal-form, .add-form');
        const bounds = owner?.getBoundingClientRect();
        return {
          label: element.getAttribute('aria-label') || element.textContent || element.id,
          left: r.left, right: r.right, width: r.width, height: r.height,
          withinOwner: !bounds || (r.left >= bounds.left - 1 && r.right <= bounds.right + 1
            && r.top >= bounds.top - 1 && r.bottom <= bounds.bottom + 1),
        };
      })
    ));
    assert.ok(controls.length > 0, 'editable page has no controls');
    for (const control of controls) {
      assert.ok(control.left >= -1 && control.right <= test.width + 1, `clipped control: ${control.label}`);
      assert.ok(control.withinOwner, `control escapes its row/form: ${control.label}`);
      assert.ok(control.width >= 24 && control.height >= 40, `control is too small: ${control.label}`);
    }
  }

  // Check only solid, opaque surfaces we can resolve without guessing about
  // gradients, decorative pseudo-elements or the SVG scene behind transparency.
  // This is a targeted control-text contrast check, not a full WCAG audit.
  let contrast = { checked: [], skipped: [] };
  if (interfaces.includes(test.theme)) {
    contrast = await page.evaluate(() => {
      const parse = (value) => {
        const match = value.match(/^rgba?\(([^)]+)\)$/);
        if (!match) return null;
        const components = match[1].split(/[,\s/]+/).map(Number);
        return { rgb: components.slice(0, 3), alpha: components[3] ?? 1 };
      };
      const luminance = (rgb) => rgb.map((channel) => {
        const v = channel / 255;
        return v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4;
      }).reduce((sum, channel, i) => sum + channel * [.2126, .7152, .0722][i], 0);
      const result = { checked: [], skipped: [] };
      const selectors = '.theme-select, .decision strong, .decision span, .toggle, .button.primary, .toast p';
      for (const element of document.querySelectorAll(selectors)) {
        if (!element.getClientRects().length) continue;
        const label = element.className || element.parentElement.className;
        let background = null;
        let uncertain = false;
        for (let current = element; current; current = current.parentElement) {
          const style = getComputedStyle(current);
          if (Number(style.opacity) !== 1) { uncertain = true; break; }
          if (background) continue;
          if (style.backgroundImage !== 'none') { uncertain = true; break; }
          const color = parse(style.backgroundColor);
          if (!color || (color.alpha > 0 && color.alpha < 1)) { uncertain = true; break; }
          if (color.alpha === 1) background = color.rgb;
          for (const pseudo of ['::before', '::after']) {
            const decoration = getComputedStyle(current, pseudo);
            if (decoration.content !== 'none' && decoration.content !== 'normal'
                && (decoration.backgroundImage !== 'none' || (parse(decoration.backgroundColor)?.alpha || 0) > 0)) {
              uncertain = true;
            }
          }
          if (uncertain) break;
        }
        const style = getComputedStyle(element);
        const foreground = parse(style.color);
        if (uncertain || !background || !foreground || foreground.alpha !== 1) {
          result.skipped.push(label);
          continue;
        }
        const a = luminance(foreground.rgb);
        const b = luminance(background);
        const ratio = (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
        const size = parseFloat(style.fontSize);
        const large = size >= 24 || (size >= 18.66 && Number(style.fontWeight) >= 700);
        result.checked.push({ label, ratio: Number(ratio.toFixed(2)), minimum: large ? 3 : 4.5 });
      }
      return result;
    });
    for (const sample of contrast.checked) {
      assert.ok(sample.ratio >= sample.minimum, `low control-text contrast: ${sample.label} (${sample.ratio}:1)`);
    }
  }

  if (test.toast) {
    const toast = page.locator('.toast');
    assert.equal(await toast.count(), 1, 'toast fixture missing');
    assert.equal(await toast.evaluate((el) => getComputedStyle(el).position), 'fixed');
    assert.equal(await toast.getAttribute('role'), test.toast === 'error' ? 'alert' : 'status');
    const before = await snapshot(page);
    const toastBefore = await toast.boundingBox();
    assert.ok(toastBefore.x >= 0 && toastBefore.x + toastBefore.width <= test.width + 1, 'toast clipped');
    // Hide only the outlet, isolating its contribution to document flow. SSR has
    // no JS timer or event handlers, so this does not claim auto-dismiss coverage.
    await toast.evaluate((el) => { el.style.display = 'none'; });
    const after = await snapshot(page);
    for (const selector of ['main', 'nav', 'footer', 'picker', 'wordmark']) {
      assert.deepEqual(after[selector], before[selector], `toast moves ${selector}`);
    }
    await toast.evaluate((el) => { el.style.display = ''; });
  }

  const animations = await page.evaluate(() => (
    [...document.querySelectorAll('*')].flatMap((element) => (
      [null, '::before', '::after'].flatMap((pseudo) => {
        const style = getComputedStyle(element, pseudo);
        const animated = style.animationName.split(',').some((name) => name.trim() !== 'none');
        const transition = style.transitionDuration.split(',').some((value) => parseFloat(value) > 0);
        return animated || transition ? [{ tag: element.tagName, class: element.className, pseudo }] : [];
      })
    ))
  ));
  assert.deepEqual(animations, [], 'reduced-motion leaves active CSS animation/transition');

  await page.evaluate(() => scrollTo(0, document.documentElement.scrollHeight / 2));
  const middle = await snapshot(page);
  await page.evaluate(() => scrollTo(0, document.documentElement.scrollHeight));
  const bottom = await snapshot(page);
  assert.equal(bottom.documentWidth, test.width, 'horizontal overflow after scrolling');
  assert.ok(bottom.footer.top >= -1 && bottom.footer.bottom <= height + 1, 'footer cannot be fully reached');
  if (test.width <= 760 || interfaces.includes(test.theme)) {
    for (const state of [top, middle, bottom]) {
      assert.equal(state.navPosition, 'fixed', 'navigation is not fixed');
      assert.ok(Math.abs(state.nav.bottom - height) <= 1, 'navigation is not at viewport bottom');
      assert.ok(Math.abs(state.nav.top - top.nav.top) <= 1, 'navigation moves when scrolling');
    }
    assert.ok(bottom.footer.bottom <= bottom.nav.top + 1, 'footer overlaps fixed navigation');
    assert.ok(bottom.nav.top - bottom.footer.bottom <= 32, 'footer has excess empty space above navigation');
  }

  const save = interfaces.includes(test.theme)
    && ((test.screen === 'eat' && test.state === 'ready' && !test.toast && [390, 1060].includes(test.width))
      || (['foods', 'history', 'backup'].includes(test.screen) && test.state === 'ready' && !test.toast && [390, 1060].includes(test.width))
      || (test.screen === 'history' && test.state === 'long')
      || test.toast === 'error');
  if (save) {
    await page.evaluate(() => scrollTo(0, 0));
    const file = path.join(output, `${browserName}-${test.key}.png`);
    await page.screenshot({ path: file, fullPage: test.screen !== 'eat' });
    report.screenshots.push(file);
  }
  return { nav_top: top.nav.top, footer_bottom: bottom.footer.bottom, scroll_y: bottom.scrollY, control_contrast: contrast };
}

async function main() {
  for (const browserName of browserNames) {
    assert.ok(['chromium', 'webkit'].includes(browserName), `Unsupported browser: ${browserName}`);
    const options = { headless: true };
    if (browserName === 'chromium') {
      const system = '/usr/bin/chromium';
      options.executablePath = process.env.CHROMIUM_PATH || (fs.existsSync(system) ? system : undefined);
      options.args = ['--no-sandbox'];
    }
    const browser = await playwright[browserName].launch(options);
    try {
      const context = await browser.newContext({ viewport: { width: 390, height }, reducedMotion: 'reduce' });
      const page = await context.newPage();
      for (const test of cases.values()) {
        const entry = { browser: browserName, ...test };
        try {
          entry.metrics = await check(page, test, browserName);
          entry.result = 'passed';
        } catch (error) {
          entry.result = 'failed';
          entry.error = error.stack || String(error);
          const file = path.join(output, `${browserName}-${test.key}-failed.png`);
          await page.screenshot({ path: file, fullPage: true }).catch(() => {});
          report.screenshots.push(file);
          console.error(`${browserName}/${test.key}: ${error.message}`);
        }
        report.cases.push(entry);
      }
    } finally {
      await browser.close();
    }
    const entries = report.cases.filter((entry) => entry.browser === browserName);
    console.log(JSON.stringify({ browser: browserName, passed: entries.filter((entry) => entry.result === 'passed').length, total: entries.length }));
  }
}

main().catch((error) => {
  report.error = error.stack || String(error);
  console.error(error);
  process.exitCode = 1;
}).finally(() => {
  report.finished_at = new Date().toISOString();
  report.passed = report.cases.filter((entry) => entry.result === 'passed').length;
  report.failed = report.cases.filter((entry) => entry.result === 'failed').length;
  fs.writeFileSync(path.join(output, 'summary.json'), JSON.stringify(report, null, 2) + '\n');
  if (report.failed) process.exitCode = 1;
  console.log(`Results and screenshots: ${output}`);
});
