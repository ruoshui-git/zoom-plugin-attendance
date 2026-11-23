import { describe, expect, it } from 'vitest';
import { nextRecordSort, sortRecordRows } from './record-sort';

describe('sortRecordRows', () => {
  const rows = [
    { id: 'b', pinyin: 'li', score: 80, firstJoinAt: '2026-01-02T00:00:00Z', totalPresenceMinutes: 20 },
    { id: 'a', pinyin: 'chen', score: 95, firstJoinAt: '2026-01-01T00:00:00Z', totalPresenceMinutes: 10 },
    { id: 'c', pinyin: 'zhang', score: 80, firstJoinAt: '', totalPresenceMinutes: 30 },
    { id: 'd', pinyin: 'li', score: 70, firstJoinAt: 'invalid', totalPresenceMinutes: 20 },
  ];

  it('sorts names by pinyin in both directions and keeps ties stable', () => {
    expect(sortRecordRows(rows, 'name', 'asc').map((row) => row.id)).toEqual(['a', 'b', 'd', 'c']);
    expect(sortRecordRows(rows, 'name', 'desc').map((row) => row.id)).toEqual(['c', 'b', 'd', 'a']);
  });
  it('sorts scores numerically in both directions', () => {
    expect(sortRecordRows(rows, 'score', 'asc').map((row) => row.id)).toEqual(['d', 'b', 'c', 'a']);
    expect(sortRecordRows(rows, 'score', 'desc').map((row) => row.id)).toEqual(['a', 'b', 'c', 'd']);
  });
  it('sorts first join times while keeping missing or invalid values last in both directions', () => {
    expect(sortRecordRows(rows, 'firstJoin', 'asc').map((row) => row.id)).toEqual(['a', 'b', 'c', 'd']);
    expect(sortRecordRows(rows, 'firstJoin', 'desc').map((row) => row.id)).toEqual(['b', 'a', 'c', 'd']);
  });
  it('sorts minutes numerically in both directions and keeps ties stable', () => {
    expect(sortRecordRows(rows, 'minutes', 'asc').map((row) => row.id)).toEqual(['a', 'b', 'd', 'c']);
    expect(sortRecordRows(rows, 'minutes', 'desc').map((row) => row.id)).toEqual(['c', 'b', 'd', 'a']);
  });
  it('returns a new array in original order when inactive', () => {
    const result = sortRecordRows(rows, '', 'desc');
    expect(result).not.toBe(rows);
    expect(result).toEqual(rows);
  });
});

describe('nextRecordSort', () => {
  it('cycles same column asc, desc, then default and starts other columns ascending', () => {
    expect(nextRecordSort({ key: '', dir: 'asc' }, 'minutes')).toEqual({ key: 'minutes', dir: 'asc' });
    expect(nextRecordSort({ key: 'minutes', dir: 'asc' }, 'minutes')).toEqual({ key: 'minutes', dir: 'desc' });
    expect(nextRecordSort({ key: 'minutes', dir: 'desc' }, 'minutes')).toEqual({ key: '', dir: 'asc' });
    expect(nextRecordSort({ key: 'minutes', dir: 'desc' }, 'name')).toEqual({ key: 'name', dir: 'asc' });
  });
  it('sorts camera minutes and mic opens with missing media last in both directions', () => {
    const media = [
      { id: 'x', cameraOnMinutes: 5, media: { unmuteCount: 2 } },
      { id: 'none' },
      { id: 'y', cameraOnMinutes: 30, media: { unmuteCount: 0 } },
    ];
    expect(sortRecordRows(media, 'camera', 'asc').map((row) => row.id)).toEqual(['x', 'y', 'none']);
    expect(sortRecordRows(media, 'camera', 'desc').map((row) => row.id)).toEqual(['y', 'x', 'none']);
    expect(sortRecordRows(media, 'mic', 'asc').map((row) => row.id)).toEqual(['y', 'x', 'none']);
    expect(sortRecordRows(media, 'mic', 'desc').map((row) => row.id)).toEqual(['x', 'y', 'none']);
  });
});
