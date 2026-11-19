import { get } from "svelte/store";
import { uiLanguage } from "./i18n-core";

export const t = (zh: string, en: string) =>
  get(uiLanguage) === "zh-CN" ? zh : en;
export const dateTime = (value: string | number | Date) =>
  new Intl.DateTimeFormat(get(uiLanguage) === "zh-CN" ? "zh-CN" : "en", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
export const timeOnly = (value: string | number | Date) =>
  new Intl.DateTimeFormat(get(uiLanguage) === "zh-CN" ? "zh-CN" : "en", {
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(value));

export type ConfirmOptions = { title: string; message: string; confirmLabel?: string; cancelLabel?: string; destructive?: boolean };
export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    window.dispatchEvent(new CustomEvent("attendflow:confirm", { detail: { ...options, confirmLabel: options.confirmLabel || t("确认", "Confirm"), cancelLabel: options.cancelLabel || t("取消", "Cancel"), resolve } }));
  });
}
