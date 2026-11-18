import { describe, expect, it } from 'vitest';
import { zoomBanner, zoomSidebarLabel, zoomSignedIn, zoomActionLabel } from './zoom-status';

const en = (_zh: string, en: string) => en;
const zh = (zh: string, _en: string) => zh;

describe('zoom status presentation', () => {
  it('hides the banner only when connected', () => {
    expect(zoomBanner({ state: 'connected' }, en)).toBeNull();
    for (const state of ['signed_out', 'signing_in', 'zoom_not_running', 'connecting', 'error', 'not_configured']) {
      expect(zoomBanner({ state }, en), state).not.toBeNull();
    }
  });

  it('offers the right action per state', () => {
    expect(zoomBanner({ state: 'signed_out' }, en)?.action).toBe('sign_in');
    expect(zoomBanner({ state: 'signed_out', reason: 'session_expired' }, en)?.title).toMatch(/expired/);
    expect(zoomBanner({ state: 'signing_in' }, en)?.action).toBe('reopen_sign_in');
    expect(zoomBanner({ state: 'error', reason: 'account_mismatch' }, en)?.action).toBe('switch_account');
    expect(zoomBanner({ state: 'zoom_not_running' }, en)?.action).toBeNull();
    expect(zoomBanner({ state: 'error', reason: 'ipc_timeout' }, en)?.body).toMatch(/same Zoom account/);
    expect(zoomBanner({ state: 'error', reason: 'auth_failed' }, en)?.action).toBeNull();
  });

  it('knows when sign-out applies', () => {
    expect(zoomSignedIn({ state: 'connected' })).toBe(true);
    expect(zoomSignedIn({ state: 'zoom_not_running' })).toBe(true);
    expect(zoomSignedIn({ state: 'signed_out' })).toBe(false);
    expect(zoomSignedIn({ state: 'signing_in' })).toBe(false);
    expect(zoomSignedIn(null)).toBe(false);
  });

  it('localizes every message', () => {
    const states = ['signed_out', 'signing_in', 'zoom_not_running', 'connecting', 'error', 'not_configured'];
    const reasons = ['', 'session_expired', 'sign_in_denied', 'sign_in_timeout', 'account_mismatch', 'missing_scope', 'network', 'ipc_timeout'];
    // Product names may stay in English; any other English word means a missing translation.
    const english = /\b(?!(?:Zoom|Workplace|AttendFlow|Desktop)\b)[A-Za-z]{4,}\b/;
    for (const state of states) for (const reason of reasons) {
      const b = zoomBanner({ state, reason }, zh)!;
      expect(b.title + b.body, `${state}/${reason}`).not.toMatch(english);
    }
    expect(zoomSidebarLabel({ state: 'connected' }, true, zh)).toBe('已连接 Zoom');
    expect(zoomSidebarLabel({ state: 'connected' }, false, en)).toBe('Starting');
    expect(zoomActionLabel('sign_in', en)).toBe('Sign in with Zoom');
  });
});
