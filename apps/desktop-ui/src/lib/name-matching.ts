export type RosterCandidate = { personId: number; canonicalName: string; pinyin?: string; active?: boolean };
export type SplitDecision = { action: string; zoomName?: string; zoomNameAtDecision?: string; splitPersonIds?: number[]; createdAt?: string };

const normalize = (value: string) => value.toLocaleLowerCase().replace(/[\s\p{P}\p{S}\u3000]/gu, '');

/** Finds people named in a shared Zoom account (for example, "Ada & Grace"). */
export function getSharedNameSplitCandidates(name: string, roster: RosterCandidate[]): RosterCandidate[] {
  const tokens = name.split(/\s*(?:&|\+|\/|、|和|及|,|，|\band\b)\s*/iu).map(normalize).filter(Boolean);
  if (tokens.length < 2) return [];
  const matched: RosterCandidate[] = [];
  for (const token of tokens) {
    const person = roster.find((entry) => entry.active !== false && (normalize(entry.canonicalName) === token || normalize(entry.pinyin || '') === token));
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
