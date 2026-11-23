import { describe, expect, it, vi } from 'vitest';
vi.mock('./ui', () => ({ t: (_zh: string, en: string) => en }));
import { manualSnapshotParticipant } from './api';

describe('manual snapshot participants', () => {
  it('stores the camera state at capture time', () => {
    const rows = [{ zoomName: 'Ada', canonicalName: 'Ada Lovelace', matchStatus: 'matched', personId: 7 }];
    expect(manualSnapshotParticipant({ userId: 1, name: 'Ada', videoOn: false }, rows)).toMatchObject({ screenName: 'Ada', matchedCanonicalName: 'Ada Lovelace', videoOn: false });
    expect(manualSnapshotParticipant({ userId: 2, name: 'Bo', videoOn: true }, rows)).toMatchObject({ videoOn: true, matchStatus: 'not-applicable' });
  });
  it('leaves the camera state out when it is unknown', () => {
    expect(manualSnapshotParticipant({ userId: 3, name: 'Cy' }, [])).not.toHaveProperty('videoOn');
  });
});
