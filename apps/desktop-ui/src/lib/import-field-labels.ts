/** Readable zh/en labels for config import preview fields (roster, name matching, profiles). */
const LABELS: Record<string, [string, string]> = {
  canonicalName: ["姓名", "Name"], pinyin: ["拼音", "Pinyin"], group: ["分组", "Group"], active: ["启用", "Active"],
  zoomName: ["昵称", "Nickname"], alias: ["昵称", "Nickname"], notes: ["备注", "Notes"], lastUsedAt: ["最后使用", "Last Used"],
  name: ["配置名称", "Profile Name"], description: ["描述", "Description"], includedSubgroups: ["包含分组", "Included Subgroups"],
  lateAfterMinutes: ["迟到阈值 (分钟)", "Late Threshold (min)"], returnWindowMinutes: ["返回窗口 (分钟)", "Return Window (min)"],
  meetingIds: ["会议 ID", "Meeting IDs"], topicKeywords: ["主题关键词", "Topic Keywords"], groupSortOrder: ["分组排序顺序", "Group Sort Order"],
};
export function importFieldLabel(key: string, t: (zh: string, en: string) => string) {
  const label = LABELS[key];
  return label ? t(label[0], label[1]) : key;
}
