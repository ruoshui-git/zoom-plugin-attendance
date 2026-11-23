import { normalizeSearchName, rowMatchesSearch } from './attendance-filters';

/** Search an event's visible names and its matched attendance row, including stored pinyin. */
export function eventMatchesSearch(event: any, rows: any[], query: string) {
  const needle = normalizeSearchName(query || '');
  if (!needle) return true;
  const names = [
    event?.screenName,
    event?.participant?.screenName,
    event?.previousScreenName,
    event?.previous_screen_name,
    event?.nextScreenName,
    event?.next_screen_name,
  ];
  if (names.some((name) => typeof name === 'string' && normalizeSearchName(name).includes(needle))) return true;

  const participant = event?.participant || event;
  const ids = [participant?.participantUUID, participant?.sessionParticipantId, participant?.entityId, event?.participantUUID, event?.sessionParticipantId];
  const screenName = participant?.screenName || event?.screenName;
  const row = rows.find((candidate) =>
    ids.some((id) => id != null && [candidate.sessionParticipantId, candidate.participantUUID, candidate.entityId].some((rowId) => rowId != null && String(rowId) === String(id)))
    || (screenName && [candidate.zoomName, candidate.canonicalName, candidate.screenName, ...(candidate.zoomNames || [])].includes(screenName))
  );
  return Boolean(row && rowMatchesSearch(row, needle));
}
