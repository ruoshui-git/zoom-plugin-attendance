<script lang="ts">
  import { onMount } from "svelte";
  import { uiLanguage, setUiLanguage } from "./lib/i18n-core";
  import {
    zoomBanner,
    zoomSidebarLabel,
    zoomActionLabel,
  } from "./lib/zoom-status";
  import {
    get,
    refresh,
    loadConfig,
    loadState,
    loadRecordEvents,
    refreshViewedRecord,
    zoomAction,
    chooseLanguage,
  } from "./lib/api";
  import {
    page,
    connected,
    meeting,
    language,
    zoom,
    record,
    stateValue,
    viewedMeeting,
  } from "./lib/stores";
  import { t, dateTime } from "./lib/ui";
  import appIcon from "./assets/app-icon.png";
  import LivePage from "./pages/LivePage.svelte";
  import SidebarNav from "./lib/SidebarNav.svelte";
  import RecordsPage from "./pages/RecordsPage.svelte";
  import ConfigurationPage from "./pages/ConfigurationPage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";
  import ConfirmDialog from "./lib/common/ConfirmDialog.svelte";
  import Toast from "./lib/common/Toast.svelte";
  import { toast } from "./lib/toast";
  import { loadWindowState } from "./lib/window-control";

  $: $language = $uiLanguage;
  $: appTitle = $language ? t("出勤助手桌面版", "AttendFlow Desktop") : "AttendFlow Desktop";
  $: banner = $language ? zoomBanner($zoom, t) : null;
  $: matchingCount =
    ($record.rows || []).filter((row: any) => row.matchStatus === "unmatched")
      .length +
    ($record.record?.nameChangeEvents || []).filter(
      (event: any) =>
        !($stateValue.reconciliationDecisions || []).some(
          (decision: any) =>
            decision.sessionParticipantId === event.sessionParticipantId &&
            Date.parse(decision.createdAt || "") >=
              Date.parse(event.timestamp || ""),
        ),
    ).length;
  onMount(() => {
    setUiLanguage(
      (localStorage.getItem("attendflow-desktop.ui-language") ||
        localStorage.getItem("ui-language") ||
        "en") as any,
    );
    get("/api/zoom")
      .then((value) => zoom.set(value))
      .catch(() => {});
    const ws = new WebSocket(`ws://${location.host}/bridge`);
    ws.onopen = () => {
      connected.set(true);
      ws.send(JSON.stringify({ type: "subscribe" }));
    };
    ws.onclose = () => connected.set(false);
    ws.onerror = () => connected.set(false);
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (message.type === "zoom_status") {
        zoom.set(message);
        return;
      }
      if (message.type === "state") {
        meeting.set(message.meeting);
        refresh();
        loadState();
        loadRecordEvents();
      }
      if (
        message.type === "record_updated" &&
        (!$meeting || message.instanceId === $meeting.instanceId)
      ) {
        refresh({ changed: [message.instanceId] });
        loadRecordEvents();
      }
      if (message.type === "record_updated" && $viewedMeeting?.instanceId === message.instanceId) {
        refreshViewedRecord(message.instanceId);
      }
    };
    refresh();
    loadConfig();
    loadWindowState();
    return () => ws.close();
  });
</script>

<svelte:head><title>{appTitle}</title></svelte:head>
<Toast toasts={$toast} onDismiss={(id) => toast.dismiss(id)} />
<ConfirmDialog />
<div class="shell">
  <aside>
    <div class="brand">
      <img class="mark" src={appIcon} alt="" width="35" height="35" />
      <div>
        <b>{t("出勤助手", "AttendFlow")}</b>
        <small class:zh={$language === "zh-CN"}>{t("桌面版", "DESKTOP")}</small>
      </div>
    </div>
    <SidebarNav {matchingCount} />
    <div class="connection">
      <i class:up={$connected && $zoom?.state === "connected"}
      ></i>{zoomSidebarLabel($zoom, $connected, t)}
    </div>
  </aside>
  <main>
    <header>
      <div>
        <div class="eyebrow">{t("会议考勤", "MEETING ATTENDANCE")}</div>
        <h1>
          {$page === "live"
            ? t("出勤", "Live Meeting")
            : $page === "records"
              ? t("考勤记录", "Records")
              : $page === "configuration"
                ? t("考勤配置", "Configuration")
                : t("其他设置", "Settings")}
        </h1>
      </div>
      <div class="header-actions">
        {#if $meeting}<span class="live-pill">
            <i></i>{$meeting.topic ||
              $meeting.meetingNumber ||
              t("会议中", "Live meeting")}
          </span>{/if}
        <select
          value={$language}
          onchange={(event) => chooseLanguage(event.currentTarget.value)}
        >
          <option value="en">English</option><option value="zh-CN">
            简体中文
          </option>
        </select>
      </div>
    </header>
    {#if banner}
      <div class="zoom-banner {banner.tone}" role="status">
        <div>
          <b>{banner.title}</b>
          <p>{banner.body}</p>
          {#if banner.detail}<small>{banner.detail}</small>{/if}
        </div>
        {#if banner.action}<button
            class="primary"
            onclick={() => banner?.action && zoomAction(banner.action)}
          >
            {zoomActionLabel(banner.action, t)}
          </button>{/if}
      </div>
    {/if}
    {#if $page === "live"}<LivePage />
    {:else if $page === "records"}<RecordsPage />
    {:else if $page === "configuration"}<ConfigurationPage />
    {:else}<SettingsPage />
    {/if}
  </main>
</div>
