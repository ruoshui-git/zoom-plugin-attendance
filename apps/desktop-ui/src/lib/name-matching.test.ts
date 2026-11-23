import { describe, expect, it } from 'vitest';
import { getSharedNameSplitCandidates, getSplitSuggestionsForZoomName } from './name-matching';

const roster = [{ personId: 1, canonicalName: 'Ada Lovelace', pinyin: 'ada' }, { personId: 2, canonicalName: 'Grace Hopper', pinyin: 'grace' }];
describe('split suggestions', () => {
  it('matches both people from a shared name', () => expect(getSharedNameSplitCandidates('Ada Lovelace & Grace Hopper', roster).map((person) => person.personId)).toEqual([1, 2]));
  it('uses the latest historical split by name', () => expect(getSplitSuggestionsForZoomName('Shared', [{ action: 'split', zoomName: 'Shared', splitPersonIds: [1, 2], createdAt: '2026-01-01' }], roster).map((person) => person.personId)).toEqual([1, 2]));
});
