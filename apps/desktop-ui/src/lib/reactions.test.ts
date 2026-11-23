import { describe, expect, it } from 'vitest';
import { deriveReactionLog, perPersonCounts, yesSince } from './reactions';

const events = [
  { kind: 'users', at: 1, users: [{ userId: 7, name: 'Ada' }] },
  { kind: 'feedback', seq: 2, at: 10, userId: 7, type: 2 },
  { kind: 'reaction', seq: 3, at: 11, userId: 7, type: 2 },
  { kind: 'feedback', seq: 4, at: 20, userId: 7, type: 0 },
];

describe('reaction log helpers', () => {
  it('derives bilingual entries and resolves names from users events', () => {
    expect(deriveReactionLog(events, [], 'zh-CN').map(({ name, kind, label }) => [name, kind, label])).toEqual([
      ['Ada', 'feedback', '是'], ['Ada', 'reaction', '点赞'], ['Ada', 'feedback', '已清除'],
    ]);
  });
  it('counts feedback by type and reactions per person', () => {
    expect(perPersonCounts(deriveReactionLog(events))).toEqual([{ name: 'Ada', feedback: { 0: 1, 2: 1 }, reactions: 1 }]);
  });
  it('only includes latest yes feedback after the check-in time', () => {
    const entries = deriveReactionLog(events);
    expect(yesSince(entries, 5)).toEqual([]);
    expect(yesSince(entries, 10)).toEqual([]);
  });
});
