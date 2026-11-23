// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import RecordNameEditor from './RecordNameEditor.svelte';
const save = vi.fn(); const success = vi.fn(); const error = vi.fn();
vi.mock('../api', () => ({ saveRecordState: (...args: any[]) => save(...args) }));
vi.mock('../busy', () => ({ runBusy: (_key: string, fn: () => any) => fn() }));
vi.mock('../toast', () => ({ toast: { success: (...args: any[]) => success(...args), error: (...args: any[]) => error(...args) } }));
describe('RecordNameEditor', () => {
 afterEach(() => { cleanup(); save.mockReset(); success.mockReset(); error.mockReset(); });
 it('saves trimmed value on Enter', async () => { save.mockResolvedValue(undefined); render(RecordNameEditor, { props: { target: 'viewed' } }); await fireEvent.click(screen.getByRole('button', { name: 'Rename record' })); const input = screen.getByRole('textbox'); await fireEvent.input(input, { target: { value: '  Sprint review  ' } }); await fireEvent.keyDown(input, { key: 'Enter' }); expect(save).toHaveBeenCalledWith({ customName: 'Sprint review' }, 'viewed'); });
 it('cancels on Escape and does not save', async () => { render(RecordNameEditor); await fireEvent.click(screen.getByRole('button', { name: 'Rename record' })); const input = screen.getByRole('textbox'); await fireEvent.input(input, { target: { value: 'discard' } }); await fireEvent.keyDown(input, { key: 'Escape' }); expect(save).not.toHaveBeenCalled(); expect(screen.queryByRole('textbox')).toBeNull(); });
 it('clears an empty name to restore the default', async () => { save.mockResolvedValue(undefined); render(RecordNameEditor, { props: { customName: 'Old title' } }); await fireEvent.click(screen.getByRole('button', { name: 'Rename record' })); const input = screen.getByRole('textbox'); await fireEvent.input(input, { target: { value: '   ' } }); await fireEvent.keyDown(input, { key: 'Enter' }); expect(save).toHaveBeenCalledWith({ customName: undefined }, 'live'); });
});
