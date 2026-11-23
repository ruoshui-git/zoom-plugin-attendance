<script lang="ts">
  import {
    meeting,
    record,
    recordEvents,
    stateValue,
    configs,
    language,
    mode,
    page,
    recordCounts,
    recordModes,
    meetings,
    settings,
    zoom,
    connected,
    selectedRecords,
    configTab,
    recordImportPreview,
    stateRevision,
    configRevision,
    stagedState,
  } from "../lib/stores";
  import {
    get,
    refresh,
    loadConfig,
    saveConfig,
    saveSettings,
    download,
    downloadBackup,
    exportRecords,
    exportOne,
    exportAll,
    deleteRecords,
    previewRecordImport,
    commitRecordImport,
    loadExampleData,
    addConfig,
    editConfig,
    removeConfig,
    loadState,
    loadRecordEvents,
    currentUsers,
    saveRecordState,
    captureManual,
    captureHands,
    applyOverrides,
    saveDecision,
    selectProfile,
    profileGuess,
    resetOverrides,
    switchMode,
    zoomSignIn,
    zoomSignOut,
    zoomAction,
    importBackup,
    chooseLanguage,
  } from "../lib/api";
  import { t, dateTime, timeOnly } from "../lib/ui";
  import { getLiveStatusLabel } from "../lib/session-labels";
  import { formatMinutes } from "../lib/formatting";
  import MeetingProfileWorkspace from "./../lib/MeetingProfileWorkspace.svelte";
  import ConfigWorkspace from "./../lib/ConfigWorkspace.svelte";
</script>

<nav class="config-nav" aria-label={t("配置页面标签", "Configuration tabs")}>
  <button
    class:active={$configTab === "roster"}
    onclick={() => ($configTab = "roster")}
  >
    {t("花名册", "Roster")}
  </button>
  <button
    class:active={$configTab === "profiles"}
    onclick={() => ($configTab = "profiles")}
  >
    {t("考勤配置", "Attendance Profiles")}
  </button>
  <button
    class:active={$configTab === "aliases"}
    onclick={() => ($configTab = "aliases")}
  >
    {t("名称匹配", "Name Matching")}
  </button>
</nav>
{#if $configTab === "profiles"}<MeetingProfileWorkspace
    profiles={$configs.profiles || []}
    revision={$configRevision.profiles || 0}
    language={$language}
    onSaved={loadConfig}
  />{:else}<ConfigWorkspace
    activeTab={$configTab === "aliases" ? "aliases" : "roster"}
    language={$language}
    exportLanguage={$settings.exportLanguage || "en"}
    roster={$configs.roster || []}
    aliases={$configs.aliases || []}
    rosterRevision={$configRevision.roster || 0}
    aliasesRevision={$configRevision.aliases || 0}
    onSaved={loadConfig}
  />{/if}

<style>
  .config-nav {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    margin: 0 0 12px;
    border-bottom: 1px solid var(--line);
  }
  .config-nav button {
    width: auto;
    flex: 0 0 auto;
    min-height: 42px;
    margin-bottom: -1px;
    padding: 10px 16px;
    border: 0;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--muted);
    font-size: 14px;
  }
  .config-nav button.active {
    border-bottom-color: var(--accent);
    color: var(--accent);
    font-weight: 650;
  }
</style>
