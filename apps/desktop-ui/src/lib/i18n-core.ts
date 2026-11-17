// Source snapshot: AttendFlow 405abd5 (2026-11-17). Adapted for zoom-companion desktop UI.
import { get, writable } from 'svelte/store';

export type UiLanguage = 'zh-CN' | 'en';
export type ExportLanguage = 'zh-CN' | 'en';

const UI_STORAGE_KEY = 'attendflow-desktop.ui-language';

function readInitialUiLanguage(): UiLanguage {
  if (typeof localStorage !== 'undefined') {
    const saved = localStorage.getItem(UI_STORAGE_KEY);
    if (saved === 'en' || saved === 'zh-CN') {
      return saved;
    }
  }

  if (typeof navigator !== 'undefined') {
    const preferredLanguages = Array.isArray(navigator.languages)
      ? navigator.languages
      : [navigator.language];
    const normalized = preferredLanguages
      .filter((value): value is string => typeof value === 'string')
      .map((value) => value.toLowerCase());

    if (normalized.some((value) => value.startsWith('zh'))) {
      return 'zh-CN';
    }
    if (normalized.some((value) => value.startsWith('en'))) {
      return 'en';
    }
  }

  return 'en';
}

export const uiLanguage = writable<UiLanguage>(readInitialUiLanguage());

// Defaults to zh-CN; Dexie persistence is wired up lazily by App.svelte
export const exportLanguage = writable<ExportLanguage>('zh-CN');

if (typeof window !== 'undefined') {
  uiLanguage.subscribe((value) => {
    window.localStorage.setItem(UI_STORAGE_KEY, value);
  });
}

export function setUiLanguage(value: UiLanguage) {
  uiLanguage.set(value);
}

export function setExportLanguage(value: ExportLanguage) {
  exportLanguage.set(value);
}

export function toggleUiLanguage() {
  uiLanguage.update((value) => (value === 'en' ? 'zh-CN' : 'en'));
}

export function pickText<T>(language: UiLanguage, zh: T, en: T) {
  return language === 'en' ? en : zh;
}

export function translate<T>(zh: T, en: T) {
  return pickText(get(uiLanguage), zh, en);
}

export function getExportLanguage(): ExportLanguage {
  return get(exportLanguage);
}

export function getRosterExportColumns(): Array<{
  key: string;
  label: string;
}> {
  const lang = getExportLanguage();
  if (lang === 'en') {
    return [
      { key: 'personId', label: 'id' },
      { key: 'canonicalName', label: 'canonicalName' },
      { key: 'pinyin', label: 'pinyin' },
      { key: 'group', label: 'group' },
      { key: 'active', label: 'active' },
      { key: 'delete', label: 'delete' },
    ];
  }
  return [
    { key: 'personId', label: 'id' },
    { key: 'canonicalName', label: '姓名' },
    { key: 'pinyin', label: '拼音' },
    { key: 'group', label: '分组' },
    { key: 'active', label: '启用' },
    { key: 'delete', label: '删除' },
  ];
}

export function getAliasExportColumns(): Array<{ key: string; label: string }> {
  const lang = getExportLanguage();
  if (lang === 'en') {
    return [
      { key: 'personId', label: 'personId' },
      { key: 'canonicalName', label: 'canonicalName' },
      { key: 'zoomName', label: 'nickname' },
      { key: 'delete', label: 'delete' },
      { key: 'lastUsedAt', label: 'lastUsedAt' },
    ];
  }
  return [
    { key: 'personId', label: '固定编号' },
    { key: 'canonicalName', label: '姓名' },
    { key: 'zoomName', label: '昵称' },
    { key: 'delete', label: '删除' },
    { key: 'lastUsedAt', label: '最后使用时间' },
  ];
}


