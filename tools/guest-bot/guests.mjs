// Guest participants for Zoom web client tests, tuned to look less like a bot.
// usage: node guests.mjs <meetingId> <pwd> "Name@startDelaySec@staySec" ...
// env: JOIN_BASE (default https://zoom.us/wc/join/), GRACEFUL=1, CHAT="text", PROFILES=<dir>, RETRIES=3,
//      ADMIT=<guest names that auto-click Admit if they become host>
import { chromium } from 'playwright';
import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const [, , MEETING_ID, PASSCODE, ...specs] = process.argv;
const HERE = path.dirname(fileURLToPath(import.meta.url));
const PROFILES = process.env.PROFILES || path.join(HERE, 'guest-profiles');
const JOIN_BASE = process.env.JOIN_BASE || 'https://zoom.us/wc/join/';
const RETRIES = +(process.env.RETRIES || 3);
const ts = () => new Date().toISOString();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const jitter = (a, b) => a + Math.random() * (b - a);
const slug = (s) => s.replace(/[^A-Za-z0-9-]/g, '_');

async function humanMouse(page) {
  for (let i = 0; i < 4; i++) {
    await page.mouse.move(jitter(100, 1100), jitter(100, 600), { steps: Math.round(jitter(8, 20)) });
    await sleep(jitter(150, 500));
  }
}

async function humanType(page, selector, text) {
  const el = page.locator(selector);
  await el.click();
  await el.fill('');
  await page.keyboard.type(text, { delay: jitter(60, 140) });
}

async function blocked(page) {
  return page.getByText(/Automated bots|bots automatizados/i).isVisible().catch(() => false);
}

async function guest(spec) {
  const [name, delay = '0', stay = '120'] = spec.split('@');
  await sleep(+delay * 1000);
  const dir = path.join(PROFILES, slug(name));
  fs.mkdirSync(dir, { recursive: true });
  const ctx = await chromium.launchPersistentContext(dir, {
    channel: 'chrome', headless: false, locale: 'en-US', viewport: { width: 1280, height: 800 },
    permissions: ['microphone', 'camera'],
    ignoreDefaultArgs: ['--enable-automation'],
    args: ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream',
      '--disable-blink-features=AutomationControlled'],
  });
  await ctx.addInitScript(() => Object.defineProperty(navigator, 'webdriver', { get: () => undefined }));
  const page = ctx.pages()[0] || (await ctx.newPage());
  let joined = false;
  try {
    // Warm-up: a normal page view builds reCAPTCHA history for this profile.
    await page.goto('https://zoom.us/', { waitUntil: 'domcontentloaded' }).catch(() => {});
    await humanMouse(page);
    await sleep(jitter(1500, 3000));
    for (let attempt = 1; attempt <= RETRIES && !joined; attempt++) {
      console.log(`${ts()} [${name}] attempt ${attempt}: navigating`);
      await page.goto(`${JOIN_BASE}${MEETING_ID}`);
      await page.waitForSelector('#input-for-name', { timeout: 30000 });
      await humanMouse(page);
      if (await page.$('#input-for-pwd')) await humanType(page, '#input-for-pwd', PASSCODE);
      await humanType(page, '#input-for-name', name);
      await sleep(jitter(800, 2000));
      await page.locator('.preview-join-button').hover();
      await sleep(jitter(200, 600));
      await page.click('.preview-join-button');
      // Outcome: blocked banner, or the preview form disappears (in meeting / waiting room).
      let outcome = 'timeout';
      for (let i = 0; i < 30 && outcome === 'timeout'; i++) {
        await sleep(1000);
        if (await blocked(page)) outcome = 'blocked';
        else if (!(await page.$('#input-for-name'))) outcome = 'joined';
      }
      if (outcome !== 'joined') {
        const wait = 20000 * attempt;
        console.log(`${ts()} [${name}] ${outcome.toUpperCase()} at join; retry in ${wait / 1000}s`);
        await page.screenshot({ path: path.join(HERE, `blocked-${slug(name)}-${attempt}.png`) });
        await sleep(wait);
        continue;
      }
      joined = true;
    }
    if (!joined) throw new Error('gave up after bot-check blocks');
    console.log(`${ts()} [${name}] JOINED (meeting or waiting room); staying ${stay}s`);
    await sleep(4000);
    await page.screenshot({ path: path.join(HERE, `shot-${slug(name)}-${Date.now()}.png`) });
    if (process.env.CHAT) {
      try {
        await page.locator('button[aria-label*="chat" i]').first().evaluate((el) => el.click());
        await sleep(2000);
        const box = page.locator('[contenteditable="true"]').first();
        await box.click({ force: true });
        await page.keyboard.type(`${process.env.CHAT} (${name})`, { delay: 50 });
        await page.keyboard.press('Enter');
        console.log(`${ts()} [${name}] sent chat`);
      } catch (e) { console.log(`${ts()} [${name}] chat failed: ${e.message.slice(0, 160)}`); }
    }
    // ADMIT=<name1,name2|*>: if this guest becomes host, keep clicking "Admit" for waiting-room arrivals.
    let admitTimer = null;
    if (process.env.ADMIT && process.env.ADMIT.split(',').includes(name)) {
      admitTimer = setInterval(async () => {
        try {
          const btns = page.locator('button:text-is("Admit"), button:text-is("Admit all")');
          const n = await btns.count();
          for (let i = 0; i < n; i++) {
            await btns.nth(i).evaluate((el) => el.click());
            console.log(`${ts()} [${name}] clicked Admit`);
          }
        } catch {}
      }, 2000);
    }
    await sleep(Math.max(0, +stay * 1000 - 4000));
    if (admitTimer) clearInterval(admitTimer);
    if (process.env.GRACEFUL) {
      try {
        await page.locator('button[aria-label="Leave"]').first().evaluate((el) => el.click(), null, { timeout: 8000 });
        await sleep(1000);
        await page.locator('button:has-text("Leave Meeting")').first().evaluate((el) => el.click(), null, { timeout: 8000 });
        console.log(`${ts()} [${name}] clicked Leave Meeting`);
        await sleep(3000);
      } catch (e) { console.log(`${ts()} [${name}] graceful leave failed: ${e.message.slice(0, 160)}`); }
    }
  } catch (e) { console.log(`${ts()} [${name}] error: ${e.message.slice(0, 200)}`); }
  console.log(`${ts()} [${name}] LEAVING (closing browser)`);
  await ctx.close();
}

await Promise.all(specs.map(guest));
console.log(`${ts()} done`);
