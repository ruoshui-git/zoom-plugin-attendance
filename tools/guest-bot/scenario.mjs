// Live-test guests driven by a control file (one command per line, appended while running).
// usage: node scenario.mjs <meetingId> <pwd> <ctlFile> <truthJsonl> "Name[@audioWav]" ...
// control lines: "<Name|*> <action>"; actions: raise, lower, audio, unmute, mute, lowerall,
//   participants, click <regex on aria-label/text>, text <exact visible text>, shot, buttons, leave,
//   quit (quit = close every browser and exit).
// Web-client notes (2026-11-23): raise hand = "click ^React$" then "click ^Raise Hand$" (Alt+Y did nothing);
// co-host "lower all" lives at Participants > More > "Clear All Feedback".
// Every action is written to <truthJsonl> with a wall-clock timestamp (ground truth for the lab log).
// Join logic mirrors guests.mjs (persistent profiles + human pacing to avoid Zoom's bot check).
import { chromium } from 'playwright';
import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const [, , MEETING_ID, PASSCODE, CTL, TRUTH, ...specs] = process.argv;
const HERE = path.dirname(fileURLToPath(import.meta.url));
const PROFILES = process.env.PROFILES || path.join(HERE, 'guest-profiles');
const JOIN_BASE = process.env.JOIN_BASE || 'https://zoom.us/wc/join/';
const SHOTS = process.env.SHOTS || path.dirname(TRUTH);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const jitter = (a, b) => a + Math.random() * (b - a);
const slug = (s) => s.replace(/[^A-Za-z0-9-]/g, '_');
const truth = (o) => { const line = JSON.stringify({ at: Date.now(), iso: new Date().toISOString(), ...o }); fs.appendFileSync(TRUTH, line + '\n'); console.log(line); };

async function humanMouse(page) {
  for (let i = 0; i < 4; i++) { await page.mouse.move(jitter(100, 1100), jitter(100, 600), { steps: Math.round(jitter(8, 20)) }); await sleep(jitter(150, 500)); }
}
async function humanType(page, selector, text) {
  const el = page.locator(selector); await el.click(); await el.fill(''); await page.keyboard.type(text, { delay: jitter(60, 140) });
}
const blocked = (page) => page.getByText(/Automated bots|bots automatizados/i).isVisible().catch(() => false);

async function clickByLabel(page, re) {
  // Click the first visible-or-not button whose aria-label or text matches; returns the matched label.
  return page.evaluate((src) => {
    const re = new RegExp(src, 'i');
    for (const el of document.querySelectorAll('button, [role="button"], [role="menuitem"], a')) {
      const label = (el.getAttribute('aria-label') || el.textContent || '').trim();
      if (re.test(label)) { el.click(); return label; }
    }
    return null;
  }, re.source);
}

async function join(name, wav) {
  const dir = path.join(PROFILES, slug(name)); fs.mkdirSync(dir, { recursive: true });
  const args = ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream', '--disable-blink-features=AutomationControlled', '--autoplay-policy=no-user-gesture-required'];
  if (wav) args.push(`--use-file-for-fake-audio-capture=${wav}`);
  const ctx = await chromium.launchPersistentContext(dir, {
    channel: 'chrome', headless: false, locale: 'en-US', viewport: { width: 1280, height: 800 },
    permissions: ['microphone', 'camera'], ignoreDefaultArgs: ['--enable-automation'], args,
  });
  await ctx.addInitScript(() => Object.defineProperty(navigator, 'webdriver', { get: () => undefined }));
  const page = ctx.pages()[0] || (await ctx.newPage());
  await page.goto('https://zoom.us/', { waitUntil: 'domcontentloaded' }).catch(() => {});
  await humanMouse(page); await sleep(jitter(1500, 3000));
  for (let attempt = 1; attempt <= 4; attempt++) {
    await page.goto(`${JOIN_BASE}${MEETING_ID}`);
    await page.waitForSelector('#input-for-name', { timeout: 30000 });
    await humanMouse(page);
    if (await page.$('#input-for-pwd')) await humanType(page, '#input-for-pwd', PASSCODE);
    await humanType(page, '#input-for-name', name);
    await sleep(jitter(800, 2000));
    await page.locator('.preview-join-button').hover(); await sleep(jitter(200, 600));
    await page.click('.preview-join-button');
    let outcome = 'timeout';
    for (let i = 0; i < 30 && outcome === 'timeout'; i++) {
      await sleep(1000);
      if (await blocked(page)) outcome = 'blocked'; else if (!(await page.$('#input-for-name'))) outcome = 'joined';
    }
    truth({ guest: name, action: 'join_attempt', attempt, outcome });
    if (outcome === 'joined') return { name, ctx, page };
    await sleep(20000 * attempt);
  }
  await ctx.close(); throw new Error(`${name}: gave up joining`);
}

const ACTIONS = {
  // Web client shortcuts: Alt+Y toggles raise hand, Alt+A toggles mute, Alt+U toggles participants panel.
  raise: async (g) => { await g.page.bringToFront(); await g.page.keyboard.press('Alt+KeyY'); },
  lower: async (g) => { await g.page.bringToFront(); await g.page.keyboard.press('Alt+KeyY'); },
  unmute: async (g) => { await g.page.bringToFront(); await g.page.keyboard.press('Alt+KeyA'); },
  mute: async (g) => { await g.page.bringToFront(); await g.page.keyboard.press('Alt+KeyA'); },
  audio: async (g) => clickByLabel(g.page, /join audio|join with computer audio|computer audio/),
  participants: async (g) => { await g.page.bringToFront(); await g.page.keyboard.press('Alt+KeyU'); },
  lowerall: async (g) => clickByLabel(g.page, /lower all hands/),
  click: async (g, rest) => clickByLabel(g.page, new RegExp(rest)),
  // Menu entries are often plain <li>/<span>: click the first element whose own text matches exactly.
  text: async (g, rest) => { await g.page.getByText(rest, { exact: true }).first().click({ timeout: 5000 }); return rest; },
  shot: async (g) => { const p = path.join(SHOTS, `shot-${slug(g.name)}-${Date.now()}.png`); await g.page.screenshot({ path: p }); return p; },
  buttons: async (g) => g.page.evaluate(() => [...document.querySelectorAll('button,[role="button"],[role="menuitem"]')].map((e) => (e.getAttribute('aria-label') || e.textContent || '').trim()).filter(Boolean).slice(0, 120)),
  leave: async (g) => {
    await clickByLabel(g.page, /^leave$/); await sleep(1200);
    return clickByLabel(g.page, /^leave meeting$/);
  },
};

const guests = new Map();
await Promise.all(specs.map(async (spec, i) => {
  const [name, wav] = spec.split('@');
  await sleep(i * 9000); // stagger joins so the bot check sees human-like pacing
  try { guests.set(name, await join(name, wav)); } catch (e) { truth({ guest: name, action: 'join_failed', error: e.message }); }
}));
truth({ action: 'all_joined', guests: [...guests.keys()] });

let offset = fs.existsSync(CTL) ? fs.statSync(CTL).size : 0;
let quit = false;
while (!quit) {
  await sleep(250);
  const size = fs.statSync(CTL).size;
  if (size <= offset) continue;
  const fd = fs.openSync(CTL, 'r'); const buf = Buffer.alloc(size - offset); fs.readSync(fd, buf, 0, buf.length, offset); fs.closeSync(fd);
  offset = size;
  for (const line of buf.toString('utf8').split(/\r?\n/).map((l) => l.trim()).filter(Boolean)) {
    const [who, action, ...restParts] = line.split(/\s+/); const rest = restParts.join(' ');
    if (action === 'quit' || who === 'quit') { quit = true; break; }
    const targets = who === '*' ? [...guests.values()] : [guests.get(who)].filter(Boolean);
    for (const g of targets) {
      const fn = ACTIONS[action];
      if (!fn) { truth({ guest: g.name, action, error: 'unknown action' }); continue; }
      truth({ guest: g.name, action, phase: 'before' });
      try { const result = await fn(g, rest); truth({ guest: g.name, action, phase: 'done', result: result ?? null }); }
      catch (e) { truth({ guest: g.name, action, phase: 'error', error: e.message.slice(0, 200) }); }
    }
  }
}
for (const g of guests.values()) await g.ctx.close().catch(() => {});
truth({ action: 'quit' });
