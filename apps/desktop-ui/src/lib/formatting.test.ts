import { describe, expect, it } from 'vitest';
import { formatMinutes, scoreBand } from './formatting';
describe('display formatting',()=>{
 it('rounds presentation minutes and handles missing values',()=>{expect(formatMinutes(5.6)).toBe('6');expect(formatMinutes(0)).toBe('0');expect(formatMinutes(undefined)).toBe('0')});
 it('uses AttendFlow score bands',()=>{expect(scoreBand(90)).toBe('excellent');expect(scoreBand(75)).toBe('good');expect(scoreBand(50)).toBe('fair');expect(scoreBand(0.1)).toBe('low');expect(scoreBand(0)).toBe('zero');expect(scoreBand(null)).toBe('unscored')});
});
