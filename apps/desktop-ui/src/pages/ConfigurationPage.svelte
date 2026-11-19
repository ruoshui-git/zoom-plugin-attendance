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
    toastMessage,
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

<div class="panel-title">
  <div class="switch">
    <button
      class:chosen={$configTab === "roster"}
      onclick={() => ($configTab = "roster")}
    >
      {t("花名册与别名", "Roster & aliases")}
    </button><button
      class:chosen={$configTab === "profiles"}
      onclick={() => ($configTab = "profiles")}
    >
      {t("考勤配置", "Profiles")}
    </button>
  </div>
</div>
{#if $configTab === "profiles"}<MeetingProfileWorkspace
    profiles={$configs.profiles || []}
    revision={$configRevision.profiles || 0}
    language={$language}
    onSaved={loadConfig}
  />{:else}<ConfigWorkspace
    language={$language}
    exportLanguage={$settings.exportLanguage || "en"}
    roster={$configs.roster || []}
    aliases={$configs.aliases || []}
    rosterRevision={$configRevision.roster || 0}
    aliasesRevision={$configRevision.aliases || 0}
    onSaved={loadConfig}
  />{/if}
