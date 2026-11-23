/** Default title and optional custom title shared by live, list and detail views. */
export function recordTitle(meeting: any) {
  const secondary = `${meeting?.topic || meeting?.meetingNumber || '—'}${meeting?.startedAt ? ` · ${meeting.startedAt}` : ''}`;
  const primary = typeof meeting?.customName === 'string' && meeting.customName.trim()
    ? meeting.customName.trim()
    : secondary;
  return { primary, secondary: primary === secondary ? '' : secondary };
}
