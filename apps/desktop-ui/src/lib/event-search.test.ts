import { describe, expect, it } from 'vitest';
import { eventMatchesSearch } from './event-search';

describe('eventMatchesSearch', () => {
  it('matches an event through pinyin stored on its participant row', () => {
    const event = { participant: { participantUUID: 'p1', screenName: '张伟' } };
    const rows = [{ participantUUID: 'p1', canonicalName: '张伟', pinyin: 'zhangwei' }];
    expect(eventMatchesSearch(event, rows, 'zhangwei')).toBe(true);
    expect(eventMatchesSearch(event, rows, 'wangwu')).toBe(false);
  });

  it('searches either old or new name in a name-change event', () => {
    const event = { previousScreenName: 'Old Name', nextScreenName: 'New Name' };
    expect(eventMatchesSearch(event, [], 'old name')).toBe(true);
    expect(eventMatchesSearch(event, [], 'new name')).toBe(true);
  });
});
