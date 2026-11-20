export type AttendanceStatus = 'seen' | 'full_present' | 'late' | 'left_early' | 'absent' | 'missing' | 'returned' | 'ignored' | 'total' | 'present';

const styles: Record<AttendanceStatus, { zh: string; en: string; shortZh: string; shortEn: string; color: string; background: string; border: string }> = {
  seen: { zh: '到过', en: 'Seen', shortZh: '到过', shortEn: 'Se', color: '#2b6cb0', background: '#ebf8ff', border: '#bee3f8' },
  full_present: { zh: '全程', en: 'Full Present', shortZh: '全程', shortEn: 'FP', color: '#38a169', background: '#ecfdf5', border: '#c6f0d9' },
  present: { zh: '全程', en: 'Full Present', shortZh: '全程', shortEn: 'FP', color: '#38a169', background: '#ecfdf5', border: '#c6f0d9' },
  late: { zh: '迟到', en: 'Late', shortZh: '迟到', shortEn: 'Lt', color: '#ed8936', background: '#fffbeb', border: '#fed7aa' },
  left_early: { zh: '早退', en: 'Left Early', shortZh: '早退', shortEn: 'LE', color: '#b7791f', background: '#fff7e6', border: '#f6d78b' },
  absent: { zh: '缺席', en: 'Absent', shortZh: '缺席', shortEn: 'A', color: '#e53e3e', background: '#fef2f2', border: '#fecaca' },
  missing: { zh: '缺席', en: 'Absent', shortZh: '缺席', shortEn: 'A', color: '#e53e3e', background: '#fef2f2', border: '#fecaca' },
  returned: { zh: '返回', en: 'Returned', shortZh: '返回', shortEn: 'R', color: '#596273', background: '#edf1f5', border: '#d7dde5' },
  ignored: { zh: '不计入', en: 'Ignored', shortZh: '不计入', shortEn: 'I', color: '#596273', background: '#edf1f5', border: '#d7dde5' },
  total: { zh: '总计', en: 'Total', shortZh: '总', shortEn: 'T', color: '#596273', background: '#edf1f5', border: '#d7dde5' },
};

export function statusStyle(status: string) {
  return styles[(status in styles ? status : 'total') as AttendanceStatus];
}

export function statusLabel(status: string, language: string, compact = false) {
  if (!(status in styles)) return language === 'zh-CN' ? '未知' : 'Unknown';
  const style = statusStyle(status);
  return language === 'zh-CN' ? (compact ? style.shortZh : style.zh) : (compact ? style.shortEn : style.en);
}
