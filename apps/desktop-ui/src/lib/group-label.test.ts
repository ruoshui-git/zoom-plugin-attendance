import { describe, expect, it } from 'vitest';
import { groupKey, groupLabel, UNGROUPED_KEY } from './group-label';

describe('group labels', () => {
  it('normalizes missing and engine sentinel groups to one key', () => {
    for (const group of ['', '  ', null, undefined, '__UNGROUPED__', ' __UNGROUPED__ ']) expect(groupKey(group)).toBe(UNGROUPED_KEY);
  });
  it('trims regular groups and localizes the ungrouped label', () => {
    expect(groupKey('  Class A  ')).toBe('Class A');
    expect(groupLabel('__UNGROUPED__', 'zh-CN')).toBe('未分组');
    expect(groupLabel('', 'en')).toBe('Ungrouped');
    expect(groupLabel('  Class A ', 'en')).toBe('Class A');
  });
});
