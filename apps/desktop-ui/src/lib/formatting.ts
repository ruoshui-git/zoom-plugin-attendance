export function formatMinutes(value: number | null | undefined): string {
  if (!Number.isFinite(value) || (value ?? 0) <= 0) return '0';
  return `${Math.round(value!)}`;
}
export function scoreBand(score: number | null | undefined): 'excellent' | 'good' | 'fair' | 'low' | 'zero' | 'unscored' {
  if (score == null || !Number.isFinite(score)) return 'unscored';
  if (score >= 90) return 'excellent';
  if (score >= 75) return 'good';
  if (score >= 50) return 'fair';
  if (score > 0) return 'low';
  return 'zero';
}
