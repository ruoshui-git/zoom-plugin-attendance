// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import ProfileOverrideForm from './ProfileOverrideForm.svelte';

afterEach(() => cleanup());

describe('live profile override form', () => {
  it('accepts HH:mm late time and a bounded return window on Apply', async () => {
    const onApply = vi.fn();
    render(ProfileOverrideForm, { language: 'en', onApply });
    await fireEvent.input(screen.getByPlaceholderText('10 or 00:10'), { target: { value: '00:12' } });
    await fireEvent.input(screen.getByLabelText('Return override (1 to 60 minutes)'), { target: { value: '9' } });
    await fireEvent.click(screen.getByText('Apply'));
    expect(onApply).toHaveBeenCalledWith({ lateAfterMinutesOverride: 12, returnWindowMinutesOverride: 9 });
  });
});
