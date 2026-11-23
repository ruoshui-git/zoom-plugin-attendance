import { describe, expect, it } from 'vitest';
import { recordTitle } from './record-title';

describe('recordTitle', () => {
  it('uses a custom name as primary and preserves the supplied default as secondary', () => {
    expect(recordTitle({ customName: '  Project Alpha ' }, 'Weekly meeting')).toEqual({ primary: 'Project Alpha', secondary: 'Weekly meeting' });
  });
  it('uses the default alone for empty and whitespace names', () => {
    expect(recordTitle({ customName: '' }, 'Weekly meeting')).toEqual({ primary: 'Weekly meeting', secondary: '' });
    expect(recordTitle({ customName: '  ' }, 'Weekly meeting')).toEqual({ primary: 'Weekly meeting', secondary: '' });
  });
});
