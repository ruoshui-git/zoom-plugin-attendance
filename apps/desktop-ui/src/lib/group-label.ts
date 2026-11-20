export const UNGROUPED_KEY = '未分组';

export function groupKey(group: unknown): string {
  if (group == null) return UNGROUPED_KEY;
  const value = String(group).trim();
  return !value || value === '__UNGROUPED__' ? UNGROUPED_KEY : value;
}

export function groupLabel(group: unknown, language: string): string {
  const key = groupKey(group);
  return key === UNGROUPED_KEY ? (language === 'zh-CN' ? '未分组' : 'Ungrouped') : key;
}
