import { describe, expect, it } from 'vitest';
import { deriveHandEntries, handEntryActionLabel, eventTypeCounts, filterEventsByType, filterMeetings, filterRowsByTag, sortRows } from './record-tools';

describe('record detail helpers', () => {
  it('filters records by date/topic search and mode/source/status', () => {
    const meetings = [
      { topic: 'Weekly staff', meetingNumber: '101', startedAt: '2025-03-15T13:00:00Z', mode: 'roster', source: 'imported', endedAt: '2025-03-15T14:00:00Z' },
      { topic: 'Student review', meetingNumber: '202', startedAt: '2025-03-16T13:00:00Z', mode: 'generic', source: 'live' },
    ];
    expect(filterMeetings(meetings, { search: '2025-03-15', mode: 'roster', source: 'imported', status: 'ended' })).toEqual([meetings[0]]);
    expect(filterMeetings(meetings, { search: 'student', status: 'active' })).toEqual([meetings[1]]);
    expect(filterMeetings(meetings, { search: 'absent' })).toEqual([]);
  });
  const rows = [
    { zoomName: 'Bee', pinyin: 'bee', score: 50, totalPresenceMinutes: 20, tags: ['seen'], hasBeenSeen: true },
    { zoomName: 'Ada', pinyin: 'ada', score: 95, totalPresenceMinutes: 4, tags: ['late'], hasBeenSeen: true, wasLate: true },
    { zoomName: 'Missing', currentStatus: 'missing', matchStatus: 'matched', tags: ['missing'] },
  ];
  it('filters attendance rows by summary tag', () => {
    expect(filterRowsByTag(rows, 'late').map((r) => r.zoomName)).toEqual(['Ada']);
    expect(filterRowsByTag(rows, 'missing').map((r) => r.zoomName)).toEqual(['Missing']);
    expect(filterRowsByTag(rows, 'total')).toHaveLength(3);
    expect(filterRowsByTag(rows, 'ignored').map((r) => r.zoomName)).toEqual([]);
  });
  it('sorts rows by name, score, join time, and presence minutes', () => {
    expect(sortRows(rows, 'name').map((r) => r.zoomName)).toEqual(['Ada', 'Bee', 'Missing']);
    expect(sortRows(rows, 'score', 'desc').map((r) => r.zoomName)).toEqual(['Ada', 'Bee', 'Missing']);
    expect(sortRows(rows, 'minutes').map((r) => r.zoomName)).toEqual(['Missing', 'Ada', 'Bee']);
  });
  it('counts event chips and deduplicates repeated raise/lower state events', () => {
    expect(eventTypeCounts([{type:'join'},{type:'join'},{type:'leave'}])).toEqual({join:2,leave:1});
    expect(filterEventsByType([{type:'join'},{type:'leave'},{type:'join'}], 'leave')).toEqual([{type:'leave'}]);
    const entries = deriveHandEntries([
      {kind:'hand',seq:1,at:10,userId:7,raised:true}, {kind:'hand',seq:2,at:11,userId:7,raised:true},
      {kind:'hand',seq:3,at:12,userId:7,raised:false}, {kind:'hand',seq:4,at:13,userId:7,raised:false},
      {kind:'hand',seq:5,at:14,userId:7,raised:true},
    ], [{userId:7,name:'Ada'}]);
    expect(entries.map((e) => e.action)).toEqual(['raise','lower','raise']);
    expect(entries[0].screenName).toBe('Ada');
    expect(deriveHandEntries([{kind:'users',seq:6,at:20,users:[{userId:7,name:'Ada',raisedHand:true}]}]).map((e) => e.action)).toEqual(['raise']);
  });
  it('derives flag lowers, leave lowers, and deduplicates matching hand events', () => {
    const events = [
      {kind:'users',action:'sync_all',seq:1,at:1,users:[{userId:1,name:'A',raisedHand:true},{userId:2,name:'B',raisedHand:true}]},
      {kind:'users',action:'update',seq:2,at:2,users:[{userId:1,name:'A',raisedHand:false}]},
      {kind:'hand',seq:3,at:3,userId:1,raised:false},
      {kind:'users',action:'left',seq:4,at:4,users:[{userId:2,name:'B'}]},
    ];
    const entries = deriveHandEntries(events);
    expect(entries.map((e) => [e.action,e.reason])).toEqual([['raise',undefined],['raise',undefined],['lower',undefined],['lower','left']]);
  });
  it('deduplicates lowers across user flags, hand events, and all-hands markers', () => {
    const events = [{kind:'users',action:'sync_all',seq:1,at:1,users:[{userId:1,raisedHand:true},{userId:2,raisedHand:true}]},
      {kind:'users',action:'update',seq:2,at:2,users:[{userId:1,raisedHand:false},{userId:2,raisedHand:false}]},
      {kind:'hand',seq:3,at:3,userId:1,raised:false},{kind:'hand',seq:4,at:4,userId:2,raised:false},
      ...[5,6,7].map((seq) => ({kind:'all_hands_lowered',seq,at:seq}))];
    expect(deriveHandEntries(events).filter((e) => e.action === 'lower')).toHaveLength(2);
  });
  it('preserves explicit left reason and localizes its label', () => {
    const [, entry] = deriveHandEntries([{kind:'hand',seq:1,at:1,userId:1,raised:true},{kind:'hand',seq:2,at:2,userId:1,raised:false,reason:'left'}]);
    expect(entry.reason).toBe('left');
    expect(handEntryActionLabel(entry,'en')).toBe('Lowered (left meeting)');
    expect(handEntryActionLabel(entry,'zh-CN')).toBe('已放下（离开会议）');
  });
});
