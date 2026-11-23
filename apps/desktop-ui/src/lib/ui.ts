import { get } from "svelte/store";
import { uiLanguage } from "./i18n-core";

export const t = (zh: string, en: string) =>
  get(uiLanguage) === "zh-CN" ? zh : en;
export const dateTime = (value: string | number | Date) =>
  new Intl.DateTimeFormat(get(uiLanguage) === "zh-CN" ? "zh-CN" : "en", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
/** 24-hour HH:mm:ss, as AttendFlow shows times in tables and logs. */
export const timeOnly = (value: string | number | Date) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "—";
  return [date.getHours(), date.getMinutes(), date.getSeconds()].map((n) => String(n).padStart(2, "0")).join(":");
};

export type ConfirmOptions = { title: string; message: string; confirmLabel?: string; cancelLabel?: string; destructive?: boolean };
export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    window.dispatchEvent(new CustomEvent("attendflow:confirm", { detail: { ...options, confirmLabel: options.confirmLabel || t("确认", "Confirm"), cancelLabel: options.cancelLabel || t("取消", "Cancel"), resolve } }));
  });
}
