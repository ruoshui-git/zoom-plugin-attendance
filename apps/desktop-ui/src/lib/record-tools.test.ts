import { describe, expect, it } from 'vitest';
import { deriveHandEntries, eventTypeCounts, filterEventsByType, filterRowsByTag, sortRows } from './record-tools';

describe('record detail helpers', () => {
  const rows = [
    { zoomName: 'Bee', pinyin: 'bee', score: 50, totalPresenceMinutes: 20, tags: ['seen'], hasBeenSeen: true },
    { zoomName: 'Ada', pinyin: 'ada', score: 95, totalPresenceMinutes: 4, tags: ['late'], hasBeenSeen: true, wasLate: true },
    { zoomName: 'Missing', currentStatus: 'missing', matchStatus: 'matched', tags: ['missing'] },
  ];
  it('filters attendance rows by summary tag', () => {
    expect(filterRowsByTag(rows, 'late').map((r) => r.zoomName)).toEqual(['Ada']);
    expect(filterRowsByTag(rows, 'missing').map((r) => r.zoomName)).toEqual(['Missing']);
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
});
