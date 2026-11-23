/** Default title and optional custom title shared by live, list and detail views. */
export function recordTitle(meeting: any, defaultTitle?: string) {
  const secondary = defaultTitle ?? (meeting?.topic || meeting?.meetingNumber || '—');
  const custom = typeof meeting?.customName === 'string' ? meeting.customName.trim() : '';
  return { primary: custom || secondary, secondary: custom ? secondary : '' };
}
