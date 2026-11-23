// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ExportMenu from './ExportMenu.svelte';
import SlideOverPanel from './SlideOverPanel.svelte';

describe('records shared controls', () => {
  afterEach(cleanup);
  it('groups the supported export formats and dispatches the selected kind', async () => {
    const onExport = vi.fn();
    render(ExportMenu, { props: { labels: { all: 'Export all', json: 'Export JSON', attendance: 'Attendance CSV', manual: 'Manual CSV' }, onExport } });
    await fireEvent.click(screen.getByRole('button', { name: /Export all/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Attendance CSV' }));
    expect(onExport).toHaveBeenCalledWith('summary');
  });
  it('closes on outside click and Escape returns focus to the toggle', async () => {
    render(ExportMenu, { props: { labels: { all: 'Export all', json: 'Export JSON', attendance: 'Attendance CSV', manual: 'Manual CSV' } } });
    const toggle = screen.getByRole('button', { name: /Export all/ });
    await fireEvent.click(toggle);
    expect(screen.getByRole('button', { name: 'Export JSON' })).toBeTruthy();
    await fireEvent.mouseDown(document.body);
    expect(screen.queryByRole('button', { name: 'Export JSON' })).toBeNull();
    await fireEvent.click(toggle);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('button', { name: 'Export JSON' })).toBeNull();
    expect(document.activeElement).toBe(toggle);
  });
  it('shows import preview content in a closable side panel', async () => {
    const onClose = vi.fn();
    render(SlideOverPanel, { props: { open: true, title: 'Import preview', closeLabel: 'Close', onClose } });
    expect(screen.getByRole('dialog', { name: 'Import preview' })).toBeTruthy();
    await fireEvent.click(screen.getAllByRole('button', { name: 'Close' })[0]);
    expect(onClose).toHaveBeenCalledOnce();
  });
});
