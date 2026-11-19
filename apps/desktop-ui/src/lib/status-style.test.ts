import { describe, expect, it } from 'vitest';
import { statusLabel, statusStyle } from './status-style';

describe('status styles', () => {
  it('uses the AttendFlow status colors and labels', () => {
    expect(statusStyle('late').color).toBe('#ed8936');
    expect(statusStyle('seen').color).toBe('#2b6cb0');
    expect(statusLabel('full_present', 'en')).toBe('Full Present');
    expect(statusLabel('left_early', 'zh-CN', true)).toBe('早退');
    expect(statusLabel('absent', 'en', true)).toBe('A');
  });
});
