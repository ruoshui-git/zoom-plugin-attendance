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
  import NameMatchingPanel from "./../lib/NameMatchingPanel.svelte";
</script>

{#if $meeting && $mode === "roster"}<NameMatchingPanel
    rows={$record.rows || []}
    decisions={$stateValue.reconciliationDecisions || []}
    roster={$configs.roster || []}
    aliases={$configs.aliases || []}
    nameChanges={$record.record?.nameChangeEvents || []}
    language={$language}
    onDecision={(d) => saveDecision(d)}
    onRevert={(id) => saveDecision(null, id)}
  />{:else}<section class="empty-card">
    <h2>
      {t(
        "名称匹配仅适用于花名册模式",
        "Name matching is available in roster mode",
      )}
    </h2>
    <button
      onclick={() => {
        $page = "live";
        switchMode("roster");
      }}
    >
      {t("切换到花名册模式", "Switch to roster mode")}
    </button>
  </section>{/if}
