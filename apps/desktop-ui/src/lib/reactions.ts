const feedbackLabels: Record<number, [string, string]> = {
  2: ['是', 'Yes'], 3: ['否', 'No'], 4: ['快一点', 'Faster'], 5: ['慢一点', 'Slower'], 9: ['我离开一下', "I'm away"], 0: ['已清除', 'Cleared'],
};
const reactionLabels: Record<number, [string, string]> = {
  1: ['鼓掌', 'Clap'], 2: ['点赞', 'Thumbs up'], 3: ['爱心', 'Heart'], 4: ['笑哭', 'Joy'], 5: ['惊讶', 'Open mouth'], 6: ['庆祝', 'Tada'],
};

export function deriveReactionLog(events: any[], users: any[] = [], language: string = 'en') {
  const names = new Map(users.map((u) => [String(u.userId), u.name || 'Unknown']));
  const zh = language.startsWith('zh');
  return events.flatMap((event) => {
    if (event.kind === 'users') for (const user of event.users || []) names.set(String(user.userId), user.name || 'Unknown');
    if (!['feedback', 'reaction'].includes(event.kind)) return [];
    const type = Number(event.type ?? event.payload?.type);
    const labels = event.kind === 'feedback' ? feedbackLabels : reactionLabels;
    if (!labels[type]) return [];
    const userId = String(event.userId ?? event.payload?.userId ?? '');
    return [{ id: `${event.seq}-${userId}`, at: event.at, userId, name: names.get(userId) || 'Unknown', kind: event.kind, type, label: labels[type][zh ? 0 : 1] }];
  });
}

export function perPersonCounts(entries: any[]) {
  const people = new Map<string, any>();
  for (const entry of entries) {
    const key = entry.name || 'Unknown';
    const person = people.get(key) || { name: key, feedback: {}, reactions: 0 };
    if (entry.kind === 'feedback') person.feedback[entry.type] = (person.feedback[entry.type] || 0) + 1;
    else if (entry.kind === 'reaction') person.reactions += 1;
    people.set(key, person);
  }
  return [...people.values()];
}

export function yesSince(entries: any[], sinceMs: number) {
  const latest = new Map<string, any>();
  for (const entry of entries) if (entry.kind === 'feedback') {
    const key = entry.userId || entry.name;
    if (!latest.has(key) || Number(entry.at) >= Number(latest.get(key).at)) latest.set(key, entry);
  }
  return [...latest.values()].filter((entry) => Number(entry.at) >= sinceMs && entry.type === 2).map((entry) => entry.name);
}
