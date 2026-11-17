import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import NameMatchingPanel from './NameMatchingPanel.svelte';

const roster = [
  { personId: 10, canonicalName: 'Ada Lovelace', pinyin: 'ada lovelace', active: true },
  { personId: 11, canonicalName: 'Grace Hopper', pinyin: 'grace hopper', active: true },
];

describe('NameMatchingPanel action visibility', () => {
  afterEach(() => cleanup());
  it('shows candidate matches and More for unmatched rows, with manual controls collapsed', () => {
    render(NameMatchingPanel, { rows: [{ entityId: 'p1', sessionParticipantId: 'p1', zoomName: 'Ada Lovelace', matchStatus: 'unmatched' }], roster, decisions: [] });
    expect(screen.getByRole('button', { name: 'Match Ada Lovelace' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'More ▾' })).toBeTruthy();
    expect(screen.queryByRole('combobox', { name: 'Select person' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Mark Guest' })).toBeNull();
  });

  it('shows Mark Guest only when there are no candidates and Split Shared only for shared names', () => {
    render(NameMatchingPanel, { rows: [
      { entityId: 'guest', zoomName: 'Unknown', matchStatus: 'unmatched' },
      { entityId: 'shared', zoomName: 'Shared', matchStatus: 'unmatched', sharedName: true },
    ], roster, decisions: [] });
    const cards = screen.getAllByRole('article');
    expect(within(cards[0]).getByRole('button', { name: 'Mark Guest' })).toBeTruthy();
    expect(within(cards[0]).queryByRole('button', { name: /Split Shared/ })).toBeNull();
    expect(within(cards[1]).getByRole('button', { name: 'Split Shared (0)' })).toBeTruthy();
  });

  it('renders resolved decisions and hides Revert in read-only mode', () => {
    render(NameMatchingPanel, { rows: [], roster, decisions: [{ id: 'd1', zoomName: 'Ada', action: 'match', canonicalName: 'Ada Lovelace' }], readOnly: true });
    expect(screen.getByText('Matched')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Revert' })).toBeNull();
  });

  it('expands searchable manual match only after More is opened', async () => {
    render(NameMatchingPanel, { rows: [{ entityId: 'p1', sessionParticipantId: 'p1', zoomName: 'Unknown', matchStatus: 'unmatched' }], roster, decisions: [] });
    expect(screen.queryByRole('textbox', { name: 'Search roster' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'More ▾' }));
    expect(screen.getByRole('textbox', { name: 'Search roster' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Confirm Match' }).hasAttribute('disabled')).toBe(true);
  });

  it('gates Keep Match on a known previous alias for name changes', () => {
    render(NameMatchingPanel, { rows: [], roster, decisions: [], nameChanges: [
      { id: 'n1', sessionParticipantId: 'p1', previousScreenName: 'Known Nick', nextScreenName: 'Renamed' },
      { id: 'n2', sessionParticipantId: 'p2', previousScreenName: 'Unknown', nextScreenName: 'Other Name' },
    ], aliases: [{ zoomName: 'Known Nick', personId: 10 }] });
    expect(screen.getAllByRole('button', { name: 'Keep Match' })).toHaveLength(1);
  });
});
// @vitest-environment happy-dom
