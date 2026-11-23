export const statusTags = ['seen', 'full_present', 'late', 'returned', 'left_early', 'missing', 'ignored'] as const;

export function filterMeetings(meetings: any[], options: { search?: string; mode?: string; source?: string; status?: string } = {}) {
  const search = (options.search || '').trim().toLocaleLowerCase();
  return meetings.filter((meeting) => {
    const text = `${meeting.topic || ''} ${meeting.meetingNumber || ''} ${meeting.startedAt || ''} ${meeting.endedAt || ''}`.toLocaleLowerCase();
    if (search && !text.includes(search)) return false;
    if (options.mode && options.mode !== 'all' && (meeting.mode || 'generic') !== options.mode) return false;
    if (options.source && options.source !== 'all' && (meeting.source || 'live') !== options.source) return false;
    if (options.status === 'ended' && !meeting.endedAt) return false;
    if (options.status === 'active' && meeting.endedAt) return false;
    return true;
  });
}

export function filterRowsByTag(rows: any[], tag: string) {
  if (!tag || tag === 'total') return rows;
  return rows.filter((r) => (r.tags || []).includes(tag) || ({
    seen: r.hasBeenSeen, full_present: r.isCurrentlyPresent && !r.wasLate,
    late: r.wasLate, returned: r.currentStatus === 'returnedAfterLongAbsence',
    left_early: r.currentStatus === 'leftEarly', missing: r.currentStatus === 'missing',
    ignored: r.matchStatus === 'guest' || r.matchStatus === 'unmatched',
  } as Record<string, boolean>)[tag]);
}

export function sortRows(rows: any[], key: string, direction: 'asc' | 'desc' = 'asc') {
  const sign = direction === 'asc' ? 1 : -1;
  return [...rows].sort((a, b) => {
    const value = (r: any) => key === 'name' ? (r.pinyin || r.canonicalName || r.zoomName || '').toLocaleLowerCase()
      : key === 'score' ? Number(r.score || 0)
      : key === 'firstJoin' ? Date.parse(r.firstJoinAt || '') || 0
      : Number(r.totalPresenceMinutes || 0);
    const av = value(a), bv = value(b);
    return (typeof av === 'string' ? av.localeCompare(bv) : av - bv) * sign;
  });
}

export function deriveHandEntries(events: any[], users: any[] = []) {
  const names = new Map(users.map((u) => [String(u.userId), u.name || 'Unknown']));
  const last = new Map<string, boolean>();
  const entries: any[] = [];
  for (const event of events) {
    if (event.kind === 'users') for (const user of event.users || []) names.set(String(user.userId), user.name || 'Unknown');
    if (event.kind === 'users' && event.action === 'left') {
      for (const user of event.users || []) {
        const id = String(user.userId ?? '');
        if (id && last.get(id) === true) addEntry(event, id, false, 'left');
        last.delete(id);
      }
      continue;
    }
    const changes = event.kind === 'hand'
      ? [{ userId: event.userId ?? event.payload?.userId, raised: Boolean(event.raised ?? event.payload?.raised), reason: event.reason ?? event.payload?.reason }]
      : event.kind === 'users' && (!event.action || ['sync_all', 'add', 'update'].includes(event.action))
        ? (event.users || []).filter((u: any) => typeof (u.raisedHand ?? u.raised_hand) === 'boolean').map((u: any) => ({ userId: u.userId, raised: Boolean(u.raisedHand ?? u.raised_hand) }))
        : [];
    for (const change of changes) {
      const id = String(change.userId ?? '');
      const raised = change.raised;
      if (!id || (last.get(id) ?? false) === raised) continue;
      addEntry(event, id, raised, change.reason);
      last.set(id, raised);
    }
  }
  return entries;

  function addEntry(event: any, id: string, raised: boolean, reason?: string) {
    entries.push({ id: `${event.seq}-${id}`, participantUUID: id, screenName: names.get(id) || 'Unknown', timestamp: new Date(event.at).toISOString(), action: raised ? 'raise' : 'lower', sequence: event.seq, receivedAt: event.at, ...(reason ? { reason } : {}) });
  }
}

export function handEntryActionLabel(entry: { action: string; reason?: string }, language: 'en' | 'zh' | string) {
  const zh = language.startsWith('zh');
  if (entry.action === 'lower' && entry.reason === 'left') return zh ? '已放下（离开会议）' : 'Lowered (left meeting)';
  return entry.action === 'raise' ? (zh ? '举手' : 'Raised hand') : (zh ? '放下手' : 'Lowered hand');
}

export function eventTypeCounts(events: any[]) {
  return events.reduce((counts: Record<string, number>, e) => { const type = e.type || e.eventType; if (type) counts[type] = (counts[type] || 0) + 1; return counts; }, {});
}

export function filterEventsByType(events: any[], type: string) {
  return type ? events.filter((event) => (event.type || event.eventType) === type) : events;
}
