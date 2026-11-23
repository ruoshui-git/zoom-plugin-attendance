export type RosterCandidate = { personId: number; canonicalName: string; pinyin?: string; active?: boolean };
export type SplitDecision = { action: string; zoomName?: string; zoomNameAtDecision?: string; splitPersonIds?: number[]; createdAt?: string };

/** Same as AttendFlow name-utils normalizeName. */
const normalize = (value: string) => (value || '').trim().toLowerCase()
  .replace(/[\s\u3000]+/g, '')
  .replace(/[()（）[\]【】{}<>《》"'“”‘’.,，。!！?？、:：;；\-_]/g, '');
const CHINESE_NAME_SPACE_PATTERN = /[\u4e00-\u9fff]+\s+[\u4e00-\u9fff]+/;

/** Same as AttendFlow name-utils isLikelySharedName. */
export function isLikelySharedName(value: string): boolean {
  return /[\/,&，、和|]|\band\b/i.test(value) || CHINESE_NAME_SPACE_PATTERN.test(value);
}

/** Same as AttendFlow name-utils parseSharedNameTokens. */
export function parseSharedNameTokens(value: string): string[] {
  if (!value) return [];
  let parts = value.split(/(?:\s*[、，,\/&|]\s*|\s+和\s+|\s+and\s+|\s+AND\s+|\s+And\s+)/g).map((part) => part.trim()).filter(Boolean);
  if (parts.length <= 1 && CHINESE_NAME_SPACE_PATTERN.test(value)) parts = value.split(/\s+/g).map((part) => part.trim()).filter(Boolean);
  return parts;
}

/** Same as AttendFlow session-engine getSharedNameSplitCandidates. */
export function getSharedNameSplitCandidates(name: string, roster: RosterCandidate[]): RosterCandidate[] {
  if (!isLikelySharedName(name)) return [];
  const tokens = parseSharedNameTokens(name).map(normalize).filter(Boolean);
  const matched: RosterCandidate[] = [];
  for (const token of tokens) {
    const person = roster.find((entry) => normalize(entry.canonicalName) === token || normalize(entry.pinyin || '') === token);
    if (person && !matched.some((entry) => entry.personId === person.personId)) matched.push(person);
  }
  return matched;
}

export function getSplitSuggestionsForZoomName(name: string, decisions: SplitDecision[], roster: RosterCandidate[]): RosterCandidate[] {
  const normalized = normalize(name);
  const latest = decisions.filter((decision) => decision.action === 'split' && (decision.splitPersonIds?.length || 0) >= 2 && normalize(decision.zoomNameAtDecision || decision.zoomName || '') === normalized)
    .sort((a, b) => (b.createdAt || '').localeCompare(a.createdAt || ''))[0];
  const people = [...new Set(latest?.splitPersonIds || [])].map((id) => roster.find((person) => person.personId === id)).filter((person): person is RosterCandidate => Boolean(person));
  return people.length >= 2 ? people : [];
}

export function getNameCandidates(name: string, roster: RosterCandidate[], aliases: { zoomName?: string; alias?: string; personId: number }[]) {
  const normalized = normalize(name);
  const ids = aliases.filter((alias) => normalize(alias.zoomName || alias.alias || '') === normalized).map((alias) => alias.personId);
  return roster.filter((person) => person.active !== false && (ids.includes(person.personId) || normalize(person.canonicalName) === normalized || normalize(person.pinyin || '') === normalized));
}
