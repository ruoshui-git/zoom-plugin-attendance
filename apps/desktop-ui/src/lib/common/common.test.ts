// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import ConfirmDialog from './ConfirmDialog.svelte';
import ToggleSwitch from './ToggleSwitch.svelte';
import { confirmDialog } from '../ui';

afterEach(() => cleanup());

describe('shared controls', () => {
  it('resolves confirmation and cancellation', async () => {
    const onResolve=vi.fn();
    const view=render(ConfirmDialog,{open:true,title:'Delete?',message:'Confirm action',confirmLabel:'Delete',cancelLabel:'Cancel',onResolve});
    await fireEvent.click(screen.getByText('Delete'));
    expect(onResolve).toHaveBeenCalledWith(true);
    view.rerender({open:true,title:'Delete?',message:'Confirm action',confirmLabel:'Delete',cancelLabel:'Cancel',onResolve});
    await fireEvent.click(screen.getByText('Cancel'));
    expect(onResolve).toHaveBeenCalledWith(false);
  });

  it('settles the promise helper from dialog actions', async () => {
    render(ConfirmDialog);
    const accepted=confirmDialog({title:'Apply?',message:'Continue?',confirmLabel:'Continue',cancelLabel:'Stay'});
    await fireEvent.click(await screen.findByText('Continue'));
    await expect(accepted).resolves.toBe(true);
    const cancelled=confirmDialog({title:'Apply?',message:'Continue?',confirmLabel:'Continue',cancelLabel:'Stay'});
    await fireEvent.click(await screen.findByText('Stay'));
    await expect(cancelled).resolves.toBe(false);
  });

  it('toggles its checked value', async () => {
    render(ToggleSwitch,{label:'Enabled',checked:false});
    const checkbox=screen.getByRole('checkbox') as HTMLInputElement;
    await fireEvent.click(checkbox);
    expect(checkbox.checked).toBe(true);
  });
});
