import { describe, expect, it } from 'vitest';
import { audioMarker, cameraCell, cameraTooltip, micCell, micTooltip } from './media-labels';

describe('media labels', () => {
  it('formats camera minutes, percent, unknown percent, and missing data', () => {
    expect(cameraCell({ cameraOnMinutes: 42, cameraOnPercent: 70 })).toEqual({ text: '42 min · 70%', sort: 42 });
    expect(cameraCell({ cameraOnMinutes: 2.4, cameraOnPercent: null }).text).toBe('2 min · —');
    expect(cameraCell({}).text).toBe('—');
  });
  it('formats mic activity in English and Chinese and handles missing values', () => {
    const row = { media: { unmuteCount: 4 }, unmutedMinutes: 6 };
    expect(micCell(row, 'en')).toBe('Mic opened 4× · 6 min unmuted');
    expect(micCell(row, 'zh')).toBe('开麦 4 次 · 6 分钟');
    expect(micCell({}, 'en')).toBe('—');
  });
  it('labels phone and no audio, with phone taking precedence', () => {
    expect(audioMarker({ media: { lastAudioType: 2 } }, 'en')).toBe('Phone');
    expect(audioMarker({ media: { lastAudioType: 2 } }, 'zh')).toBe('电话');
    expect(audioMarker({ media: { everNoAudio: true } }, 'en')).toBe('No audio');
    expect(audioMarker({ media: { everNoAudio: true } }, 'zh')).toBe('无音频');
    expect(audioMarker({}, 'en')).toBe('');
  });
  it('provides bilingual explanations', () => {
    expect(micTooltip('en')).toContain('not speech detection');
    expect(micTooltip('zh')).toContain('并非检测说话');
    expect(cameraTooltip('en')).toContain('frozen or virtual camera');
    expect(cameraTooltip('zh')).toContain('冻结画面');
  });
});
