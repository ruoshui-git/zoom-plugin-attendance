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
