import { describe, expect, it } from 'vitest';
import { applyFilterDragEnd, filterAttendanceRows, reorderItems, sortAttendanceRows, sortGroupsByProfilePatterns } from './attendance-filters';

const rows = [
  { canonicalName: '乙', pinyin: 'yi', currentStatus: 'present', group: 'B', isExpected: true, tags: ['seen'] },
  { canonicalName: '甲', pinyin: 'jia', currentStatus: 'present', group: 'A', isExpected: false, tags: ['seen'] },
  { canonicalName: '丁', pinyin: 'ding', currentStatus: 'late', group: 'A', isExpected: true, tags: ['late'] },
];

describe('attendance filters', () => {
  it('section order moves group priority ahead of roster scope', () => {
    const expectedFirst = sortAttendanceRows(rows, { sectionOrder: ['expectation', 'group'], groups: ['A','B'], selectedGroups: ['A','B'] });
    const groupFirst = sortAttendanceRows(rows, { sectionOrder: ['group', 'expectation'], groups: ['B','A'], selectedGroups: ['A','B'] });
    expect(expectedFirst[0].canonicalName).toBe('丁');
    expect(groupFirst[0].canonicalName).toBe('乙');
    expect(groupFirst.map((r) => r.canonicalName)).not.toEqual(expectedFirst.map((r) => r.canonicalName));
  });
  it('chip order changes group row order and deselection hides rows', () => {
    expect(sortAttendanceRows(rows, { sectionOrder:['group'], groups:['A','B'], selectedGroups:['A','B'] })[0].group).toBe('A');
    expect(sortAttendanceRows(rows, { sectionOrder:['group'], groups:['B','A'], selectedGroups:['A','B'] })[0].group).toBe('B');
    expect(filterAttendanceRows(rows, { roster:true, groups:['A'] })).toHaveLength(2);
  });
  it('status preset filters to one status and base order is status then pinyin', () => {
    expect(filterAttendanceRows(rows, { status:'late' }).map((r) => r.canonicalName)).toEqual(['丁']);
    expect(sortAttendanceRows(rows, { sectionOrder:[] }).map((r) => r.canonicalName)).toEqual(['甲','乙','丁']);
  });
  it('searches current and historical names independently with normalized text', () => {
    const searchRows = [
      { canonicalName: 'Ada Lovelace', zoomName: '新名字', zoomNames: ['旧名字', '新名字'] },
      { canonicalName: "O'Neil", zoomName: 'Other' },
      { canonicalName: 'Alpha', zoomName: 'Beta' },
    ];
    expect(filterAttendanceRows(searchRows, { search: '旧名' })).toEqual([searchRows[0]]);
    expect(filterAttendanceRows(searchRows, { search: 'ada lovelace' })).toEqual([searchRows[0]]);
    expect(filterAttendanceRows(searchRows, { search: 'adalovelace' })).toEqual([searchRows[0]]);
    expect(filterAttendanceRows(searchRows, { search: 'oneil' })).toEqual([searchRows[1]]);
    expect(filterAttendanceRows(searchRows, { search: 'AlphaBeta' })).toEqual([]);
    expect(filterAttendanceRows(searchRows, { search: '' })).toEqual(searchRows);
    expect(filterAttendanceRows(searchRows, { search: '　   ' })).toEqual(searchRows);
  });
  it('seeds chip order from profile patterns', () => {
    expect(sortGroupsByProfilePatterns(['Blue','Red','Green'], ['Red','Blue'])).toEqual(['Red','Blue','Green']);
  });
  it('reorders items by pointer drag target index', () => {
    expect(reorderItems(['expectation','group'], 1, 0)).toEqual(['group','expectation']);
  });
  it('keeps a hidden expectation section in its slot when visible sections reorder', () => {
    const result = applyFilterDragEnd({ sections: ['expectation','group','other'], groups: ['A','B'], expectations: ['expected','unexpected'] }, { source: { type: 'filter-section' } }, (items) => [...items].reverse(), ['group','other']);
    expect(result.sections).toEqual(['expectation','other','group']);
  });
});
