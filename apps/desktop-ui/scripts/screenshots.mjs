import { chromium } from 'file:///C:/Users/Ruoshui/code/zoom-tester-bot/node_modules/playwright/index.mjs';
import { mkdir, readdir, unlink } from 'node:fs/promises';
import path from 'node:path';

const base = process.env.ATTENDFLOW_URL || 'http://127.0.0.1:47820/';
const out = path.resolve('apps/desktop-ui/screenshots');
await mkdir(out, { recursive: true });
for (const file of await readdir(out)) if (file.endsWith('.png')) await unlink(path.join(out, file));
const browser = await chromium.launch({ headless: true });
const names = { en: 'English', zh: '简体中文' };
const viewports = [{ width: 1280, height: 800 }, { width: 1024, height: 700 }];
const recordTabs = ['overview', 'auto-attendance', 'name-matching', 'attendance-settings', 'name-changes', 'entry-exit', 'raise-hand', 'manual-attendance-records'];

for (const viewport of viewports) {
  const page = await browser.newPage({ viewport });
  page.on('dialog', dialog => dialog.accept());
  await page.goto(base, { waitUntil: 'networkidle' });
  await page.waitForTimeout(700);
  // Populate the fake account with realistic roster and profile data.
  await page.locator('aside nav button').nth(4).click();
  await page.getByRole('button', { name: /Load example data|加载示例数据/ }).click();
  await page.waitForTimeout(500);
  const meetings = await page.evaluate(async () => (await fetch('/api/meetings')).json());
  if (meetings[0]) {
    const stateResponse = await page.evaluate(async (id) => {
      const d = await (await fetch(`/api/meetings/${id}/state`)).json();
      const value = { ...d.value,
        manualSnapshots: [{ id: 'screenshot-manual', timestamp: new Date().toISOString(), participants: [{ screenName: 'Ada Lovelace', location: 'meeting', matchedCanonicalName: 'Ada Lovelace', matchStatus: 'matched' }] }],
        raiseHandCaptures: [{ id: 'screenshot-hands', timestamp: new Date().toISOString(), participantUUIDs: ['102'], screenNames: ['毛若水'], totalCount: 1 }]
      };
      await fetch(`/api/meetings/${id}/state`, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ revision: d.revision, value }) });
      return true;
    }, meetings[0].instanceId);
  }

  for (const [locale, lang] of Object.entries(names)) {
    await page.goto(base, { waitUntil: 'networkidle' });
    await page.locator('header select').selectOption(locale === 'zh' ? 'zh-CN' : 'en');
    await page.waitForTimeout(500);
    const prefix = `${viewport.width}x${viewport.height}-${locale}`;
    const shot = async (name) => {
      await page.waitForTimeout(120);
      const size = await page.evaluate(() => ({ width: document.documentElement.scrollWidth, height: document.documentElement.scrollHeight, innerWidth: innerWidth }));
      if (size.width > size.innerWidth + 1) throw new Error(`Horizontal page overflow on ${prefix}-${name}: ${JSON.stringify(size)}`);
      await page.screenshot({ path: path.join(out, `${prefix}-${name}.png`), fullPage: false });
    };
    const nav = page.locator('aside nav button');

    await nav.nth(0).click();
    await shot('live-generic');
    const rosterMode = page.locator('.meet-head .switch button').nth(1);
    await rosterMode.click();
    await page.waitForTimeout(300);
    await shot('live-roster');

    await nav.nth(1).click();
    await shot('name-matching');

    await nav.nth(2).click();
    await shot('records-list');
    const open = page.locator('.record-open').first();
    if (await open.count()) {
      await open.click();
      await page.waitForTimeout(300);
      const tabs = page.locator('.activity-tabs button');
      const tabTotal = await tabs.count();
      for (let i = 0; i < tabTotal; i++) {
        const label = recordTabs[i];
        await tabs.nth(i).click();
        await shot(`record-${label}`);
      }
    }

    await nav.nth(3).click();
    await shot('configuration-roster');
    await page.locator('.config-tabs button').nth(1).click();
    await shot('configuration-aliases');
    await page.locator('.config-workspace .panel-title button.primary').click();
    await shot('configuration-alias-editor');
    await page.locator('.slide-panel button').filter({ hasText: /Cancel|取消/ }).last().click();
    await page.locator('.config-tabs button').first().click();
    await page.getByRole('button', { name: /Paste CSV|粘贴 CSV/ }).click();
    await shot('configuration-csv-paste');
    await page.locator('.slide-panel textarea').fill('canonicalName,pinyin,group,active\nZoe Sample,zoesample,Blue,true');
    await page.getByRole('button', { name: /Preview|预览/ }).click();
    await page.waitForTimeout(200);
    if (await page.locator('.slide-panel.import-panel').count()) await shot('configuration-import-preview');
    await page.getByRole('button', { name: /Back|返回/ }).last().click();
    if (await page.locator('.slide-panel.import-panel').count()) await page.getByRole('button', { name: /Close|关闭/ }).last().click();
    await page.locator('.panel-title .switch button').nth(1).click();
    await shot('configuration-profiles');
    await page.locator('.profile-workspace .panel-title button.primary').click();
    await shot('configuration-profile-editor');
    await page.locator('.slide-panel button').filter({ hasText: /Cancel|取消/ }).last().click();
    const profileFile = page.locator('input[type="file"]').last();
    await profileFile.setInputFiles({ name: 'profiles.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify([{ id: 'screenshot-profile', name: 'Screenshot profile', description: 'Preview only', includedSubgroups: ['*'], lateAfterMinutes: 10, returnWindowMinutes: 8, meetingIds: [], topicKeywords: [], groupSortOrder: [], active: true }])) });
    await page.waitForTimeout(250);
    if (await page.locator('.slide-panel.import-panel').count()) {
      await shot('configuration-profile-import-preview');
      await page.locator('.slide-panel button').filter({ hasText: /Cancel|取消/ }).last().click();
    }
    await shot('configuration-profile-bulk');

    await nav.nth(4).click();
    await shot('settings');
  }
  await page.close();
}
await browser.close();
console.log(`Screenshots saved to ${out}`);

