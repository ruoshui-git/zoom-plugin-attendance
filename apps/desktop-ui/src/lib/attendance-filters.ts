export const DEFAULT_FILTER_SECTION_ORDER = ['expectation', 'group'];
export const STATUS_ORDER = ['present', 'late', 'returnedAfterLongAbsence', 'leftEarly', 'missing'];
import { groupKey } from './group-label';

export function sortGroupsByProfilePatterns(groups: string[], patterns: string[] = []) {
  const matches = (pattern: string, group: string) => {
    const segments = pattern.trim().split('*'); const startsWildcard = pattern.trim().startsWith('*'); const endsWildcard = pattern.trim().endsWith('*');
    let cursor = 0;
    for (let i = 0; i < segments.length; i++) {
      const segment = segments[i]; if (!segment) continue;
      if (i === 0 && !startsWildcard) { if (!group.startsWith(segment)) return false; cursor = segment.length; continue; }
      const found = group.indexOf(segment, cursor); if (found < 0) return false; cursor = found + segment.length;
      if (i === segments.length - 1 && !endsWildcard) return cursor === group.length;
    }
    return endsWildcard || cursor === group.length;
  };
  const rank = (group: string) => patterns.findIndex((pattern) => matches(pattern, group));
  return [...groups].sort((a, b) => {
    const ar = rank(a), br = rank(b);
    if (ar >= 0 && br >= 0 && ar !== br) return ar - br;
    if (ar >= 0 && br < 0) return -1;
    if (ar < 0 && br >= 0) return 1;
    return a.localeCompare(b, 'zh-CN');
  });
}

export function reorderItems<T>(items: T[], from: number, to: number) {
  if (from < 0 || to < 0 || from >= items.length || to >= items.length || from === to) return [...items];
  const next = [...items]; const [item] = next.splice(from, 1); next.splice(to, 0, item); return next;
}

/** Merge a reordered visible subset back into its original slots. */
export function mergeVisibleOrder<T>(all: T[], visible: T[], reorderedVisible: T[]) {
  const visibleSet = new Set(visible);
  let index = 0;
  return all.map((item) => visibleSet.has(item) ? reorderedVisible[index++] : item);
}

/** Pure order update used by the filter panel's dnd-kit onDragEnd handler. */
export function applyFilterDragEnd(orders: { sections: string[]; groups: string[]; expectations: string[] }, event: any, useMove: (items: {id:string}[], event: any) => {id:string}[], visibleSections = orders.sections) {
  if (event?.canceled) return { sections: [...orders.sections], groups: [...orders.groups], expectations: [...orders.expectations] };
  const source = event?.operation?.source ?? event?.source;
  const key = source?.type === 'filter-section' ? 'sections' : source?.type === 'filter-group' ? 'groups' : source?.type === 'filter-expectation' ? 'expectations' : null;
  if (!key) return { sections: [...orders.sections], groups: [...orders.groups], expectations: [...orders.expectations] };
  const current = key === 'sections' ? visibleSections : orders[key];
  const moved = useMove(current.map((id) => ({ id })), event).map((item) => item.id);
  return { ...orders, [key]: key === 'sections' ? mergeVisibleOrder(orders.sections, visibleSections, moved) : moved };
}

/** Apply a dnd-kit sortable event to a filter order, preserving hidden section slots. */
export function reorderFilterOrder<T extends { id: string }>(order: string[], visibleOrder: string[], event: any, useMove: (items: T[], event: any) => T[]) {
  const visible = visibleOrder.map((id) => ({ id })) as T[];
  const reordered = useMove(visible, event).map((item) => item.id);
  return mergeVisibleOrder(order, visibleOrder, reordered);
}

export function normalizeSearchName(value: string) {
  return value.trim().toLowerCase().replace(/[\s　]+/g, '').replace(/[()（）[\]【】{}<>《》"'“”‘’.,，。!！?？、:：;；\-_]/g, '');
}

export function sortAttendanceRows(rows: any[], options: { sectionOrder?: string[]; groups?: string[]; expectations?: string[]; selectedGroups?: string[]; selectedExpectations?: string[]; ascending?: boolean } = {}) {
  const { sectionOrder = DEFAULT_FILTER_SECTION_ORDER, groups = [], expectations = ['expected', 'unexpected'], selectedGroups = [], selectedExpectations = ['expected', 'unexpected'], ascending = true } = options;
  return [...rows].sort((a, b) => {
    for (const section of sectionOrder) {
      const aKey = section === 'group' ? groupKey(a.group) : (a.isExpected === false ? 'unexpected' : 'expected');
      const bKey = section === 'group' ? groupKey(b.group) : (b.isExpected === false ? 'unexpected' : 'expected');
      const order = section === 'group' ? groups : expectations;
      const selected = section === 'group' ? selectedGroups : selectedExpectations;
      const diff = Number(selected.includes(bKey)) - Number(selected.includes(aKey)) || order.indexOf(aKey) - order.indexOf(bKey);
      if (diff) return diff;
    }
    const statusDiff = STATUS_ORDER.indexOf(a.currentStatus) - STATUS_ORDER.indexOf(b.currentStatus);
    if (statusDiff) return (STATUS_ORDER.indexOf(a.currentStatus) < 0 ? 999 : STATUS_ORDER.indexOf(a.currentStatus)) - (STATUS_ORDER.indexOf(b.currentStatus) < 0 ? 999 : STATUS_ORDER.indexOf(b.currentStatus));
    return (a.pinyin || a.canonicalName || a.zoomName || '').localeCompare(b.pinyin || b.canonicalName || b.zoomName || '', 'zh-CN') * (ascending ? 1 : -1);
  });
}

export function filterAttendanceRows(rows: any[], options: { status?: string; groups?: string[]; expectations?: string[]; search?: string; roster?: boolean } = {}) {
  const needle = normalizeSearchName(options.search || '');
  return rows.filter((row) => {
    if (options.status && options.status !== 'total' && !(row.tags || []).includes(options.status)) return false;
    if (options.roster && options.groups && !options.groups.includes(groupKey(row.group))) return false;
    if (options.roster && options.expectations && !options.expectations.includes(row.isExpected === false ? 'unexpected' : 'expected')) return false;
    if (!needle) return true;
    const names = [row.canonicalName, row.zoomName, row.pinyin, row.zoomPinyin, row.canonicalPinyin, ...(Array.isArray(row.zoomNames) ? row.zoomNames : [])];
    return names.some((name) => typeof name === 'string' && name.length > 0 && normalizeSearchName(name).includes(needle));
  });
}
