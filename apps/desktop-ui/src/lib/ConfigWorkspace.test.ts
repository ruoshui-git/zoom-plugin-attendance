// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, cleanup } from '@testing-library/svelte';
import ConfigWorkspace from './ConfigWorkspace.svelte';

describe('configuration import preview component', () => {
  afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

  it('sends pasted CSV to Rust and renders the returned row preview', async () => {
    const preview = {
      revision: 4,
      changes: [{ row: 2, action: 'Add', id: 3, before: null, after: { canonicalName: 'Ada' }, notes: '', aliasImpact: 0 }],
      value: [{ personId: 3, canonicalName: 'Ada', pinyin: 'ada', group: '', active: true }],
    };
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => preview });
    vi.stubGlobal('fetch', fetchMock);
    render(ConfigWorkspace, { language: 'en', roster: [], aliases: [] });
    await fireEvent.click(screen.getByText('Paste CSV'));
    await fireEvent.input(screen.getByPlaceholderText('Paste CSV content…'), { target: { value: 'canonicalName\nAda' } });
    await fireEvent.click(screen.getByText('Preview'));
    expect(fetchMock).toHaveBeenCalledWith('/api/config/roster/import/preview', expect.objectContaining({ method: 'POST' }));
    expect(await screen.findByText('Add')).toBeTruthy();
    expect(document.body.textContent).toContain('canonicalName');
    expect(document.body.textContent).toContain('"Ada"');
  });
});
