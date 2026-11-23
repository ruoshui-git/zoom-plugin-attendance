import { describe, expect, it } from 'vitest';
import { getSharedNameSplitCandidates, getSplitSuggestionsForZoomName, parseSharedNameTokens } from './name-matching';

const roster = [{ personId: 1, canonicalName: 'Ada Lovelace', pinyin: 'ada' }, { personId: 2, canonicalName: 'Grace Hopper', pinyin: 'grace' }];
describe('split suggestions', () => {
  it('matches both people from a shared name', () => expect(getSharedNameSplitCandidates('Ada Lovelace & Grace Hopper', roster).map((person) => person.personId)).toEqual([1, 2]));
  it('uses the latest historical split by name', () => expect(getSplitSuggestionsForZoomName('Shared', [{ action: 'split', zoomName: 'Shared', splitPersonIds: [1, 2], createdAt: '2026-01-01' }], roster).map((person) => person.personId)).toEqual([1, 2]));
  it('parses separators like AttendFlow', () => {
    expect(parseSharedNameTokens('张三 李四')).toEqual(['张三', '李四']);
    expect(parseSharedNameTokens('Ada | Grace')).toEqual(['Ada', 'Grace']);
    expect(parseSharedNameTokens('张三 和 李四')).toEqual(['张三', '李四']);
    expect(parseSharedNameTokens('王和平')).toEqual(['王和平']);
  });
  it('does not split a single name that contains 和', () => expect(getSharedNameSplitCandidates('王和平', [{ personId: 3, canonicalName: '王' }, { personId: 4, canonicalName: '平' }])).toEqual([]));
});
