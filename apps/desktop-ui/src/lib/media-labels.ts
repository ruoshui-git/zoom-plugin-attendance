export type MediaRow = {
  cameraOnMinutes?: number | null;
  cameraOnPercent?: number | null;
  unmutedMinutes?: number | null;
  media?: { unmuteCount?: number | null; lastAudioType?: number | null; everNoAudio?: boolean | null } | null;
};

export function cameraCell(row: MediaRow): { text: string; sort: number } {
  const minutes = row.cameraOnMinutes;
  if (minutes == null || !Number.isFinite(minutes)) return { text: '—', sort: -1 };
  const percent = row.cameraOnPercent;
  return { text: `${Math.round(minutes)} min · ${percent == null || !Number.isFinite(percent) ? '—' : `${Math.round(percent)}%`}`, sort: minutes };
}

export function micCell(row: MediaRow, lang: string): string {
  const count = row.media?.unmuteCount;
  const minutes = row.unmutedMinutes;
  if (count == null || minutes == null || !Number.isFinite(count) || !Number.isFinite(minutes)) return '—';
  return lang === 'zh' ? `开麦 ${count} 次 · ${Math.round(minutes)} 分钟` : `Mic opened ${count}× · ${Math.round(minutes)} min unmuted`;
}

export function audioMarker(row: MediaRow, lang: string): string {
  if (row.media?.lastAudioType === 2) return lang === 'zh' ? '电话' : 'Phone';
  if (row.media?.everNoAudio) return lang === 'zh' ? '无音频' : 'No audio';
  return '';
}

export const micTooltip = (lang: string) => lang === 'zh'
  ? '统计麦克风取消静音的次数和时长，并非检测说话：从不静音的人会整段计为开麦。'
  : 'Counts how often the microphone was unmuted and for how long. It is not speech detection: people who never mute count as unmuted the whole time.';

export const cameraTooltip = (lang: string) => lang === 'zh'
  ? '在会期间开启摄像头的分钟数。应用第一次看到此人之前的时间不计入。冻结画面或虚拟摄像头也算开启。'
  : 'Minutes with camera on while in the meeting. Before the app first saw a person, nothing is counted. A frozen or virtual camera counts as on.';
