import { t } from "./ui";

const messages: Record<string, [string, string]> = {
  no_active_meeting: ["没有进行中的会议。", "No active meeting."],
  busy: ["正在截图，请稍候。", "A capture is already running."],
  not_connected: ["尚未连接 Zoom。", "Zoom is not connected."],
  window_not_found: ["找不到 Zoom 会议窗口。", "Could not find the Zoom meeting window."],
  window_minimized: ["Zoom 会议窗口已最小化，请先还原后再试。", "The Zoom meeting window is minimized. Restore it and try again."],
  capture_unsupported: ["此 Windows 版本不支持窗口截图。", "Window capture is not supported on this version of Windows."],
};

/** Localized text for a capture error code from the server (also stored on failed captures). */
export function proofErrorMessage(code: string, detail?: string) {
  const pair = messages[code];
  return pair ? t(...pair) : `${t("截图失败", "Capture failed")}${detail ? `: ${detail}` : "."}`;
}

export async function captureProof() {
  const response = await fetch("/api/live/proof", { method: "POST" });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw Error(proofErrorMessage(data.error, data.detail));
  }
  return data as { captureId: string; pages: number };
}

export const proofImageUrl = (instanceId: string, file: string) => `/api/meetings/${encodeURIComponent(instanceId)}/proof/${encodeURIComponent(file)}`;

export function flattenProofPages(state: any) {
  return (state?.proofCaptures || []).slice().reverse().flatMap((capture: any) =>
    (capture.pages || []).map((page: any, index: number) => ({ ...page, captureId: capture.id, startedAt: capture.startedAt, captureError: capture.error, pageNumber: index + 1, pageCount: capture.pages.length })),
  );
}
