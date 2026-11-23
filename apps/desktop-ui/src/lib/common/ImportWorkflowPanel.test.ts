// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import ImportWorkflowPanel from './ImportWorkflowPanel.svelte';

afterEach(cleanup);

describe('ImportWorkflowPanel', () => {
  it('closes on Escape', async () => {
    const onClose = vi.fn();
    render(ImportWorkflowPanel, { props: { open: true, title: 'Import', onClose } });
    expect(screen.getByRole('dialog', { name: 'Import' })).toBeTruthy();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledOnce();
  });
});
