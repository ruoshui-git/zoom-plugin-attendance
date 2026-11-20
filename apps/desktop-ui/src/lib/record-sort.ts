export type RecordSortKey = 'name' | 'score' | 'firstJoin' | 'minutes';
export type RecordSortDir = 'asc' | 'desc';

type SortableRecordRow = {
  pinyin?: string | null;
  canonicalName?: string | null;
  zoomName?: string | null;
  score?: number | null;
  firstJoinAt?: string | null;
  totalPresenceMinutes?: number | null;
};

export function sortRecordRows<T extends SortableRecordRow>(rows: T[], key: RecordSortKey | '', dir: RecordSortDir): T[] {
  if (!key) return rows.slice();
  return rows.map((row, index) => ({ row, index })).sort((a, b) => {
    if (key === 'firstJoin') {
      const aTime = Date.parse(a.row.firstJoinAt || '');
      const bTime = Date.parse(b.row.firstJoinAt || '');
      const aValid = Number.isFinite(aTime);
      const bValid = Number.isFinite(bTime);
      if (aValid !== bValid) return aValid ? -1 : 1;
      if (!aValid) return a.index - b.index;
      const result = aTime - bTime;
      return result ? (dir === 'desc' ? -result : result) : a.index - b.index;
    }
    let result = 0;
    if (key === 'name') result = String(a.row.pinyin || a.row.canonicalName || a.row.zoomName || '').localeCompare(String(b.row.pinyin || b.row.canonicalName || b.row.zoomName || ''), 'zh-CN');
    else if (key === 'score') result = (a.row.score ?? 0) - (b.row.score ?? 0);
    else result = (a.row.totalPresenceMinutes ?? 0) - (b.row.totalPresenceMinutes ?? 0);
    return result ? (dir === 'desc' ? -result : result) : a.index - b.index;
  }).map(({ row }) => row);
}

export function nextRecordSort(current: { key: RecordSortKey | ''; dir: RecordSortDir }, clicked: RecordSortKey): { key: RecordSortKey | ''; dir: RecordSortDir } {
  if (current.key !== clicked) return { key: clicked, dir: 'asc' };
  if (current.dir === 'asc') return { key: clicked, dir: 'desc' };
  return { key: '', dir: 'asc' };
}
