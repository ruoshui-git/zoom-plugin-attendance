export type RecordSortKey = 'name' | 'score' | 'firstJoin' | 'minutes' | 'camera' | 'mic';
export type RecordSortDir = 'asc' | 'desc';

type SortableRecordRow = {
  pinyin?: string | null;
  canonicalName?: string | null;
  zoomName?: string | null;
  score?: number | null;
  firstJoinAt?: string | null;
  totalPresenceMinutes?: number | null;
  cameraOnMinutes?: number | null;
  media?: { unmuteCount?: number | null } | null;
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
    if (key === 'camera' || key === 'mic') {
      // Rows without media data (e.g. imported AttendFlow records) stay last in both directions.
      const pick = (row: T) => key === 'camera' ? row.cameraOnMinutes : row.media?.unmuteCount;
      const av = pick(a.row), bv = pick(b.row);
      const aKnown = typeof av === 'number', bKnown = typeof bv === 'number';
      if (aKnown !== bKnown) return aKnown ? -1 : 1;
      if (!aKnown) return a.index - b.index;
      const diff = (av as number) - (bv as number);
      return diff ? (dir === 'desc' ? -diff : diff) : a.index - b.index;
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
