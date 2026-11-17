// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import MeetingProfileWorkspace from './MeetingProfileWorkspace.svelte';

afterEach(() => cleanup());

describe('attendance profile workspace', () => {
  it('shows required name validation and locks the default profile name', async () => {
    render(MeetingProfileWorkspace, { profiles: [{ id: 'default-all', name: 'All participants', includedSubgroups: [], lateAfterMinutes: 10, returnWindowMinutes: 8, active: true }], language: 'en' });
    await fireEvent.click(screen.getByText('＋ Add'));
    const name = screen.getByLabelText('Name (required)') as HTMLInputElement;
    expect(name.disabled).toBe(false);
    await fireEvent.click(screen.getByText('Save'));
    expect(screen.getByText('Name is required')).toBeTruthy();
    await fireEvent.click(screen.getByText('×'));
    await fireEvent.click(screen.getByText('Edit'));
    expect((screen.getByLabelText('Name (required)') as HTMLInputElement).disabled).toBe(true);
  });
});
