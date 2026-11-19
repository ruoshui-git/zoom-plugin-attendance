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
    loadExampleDataQuiet,
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
  import { zoomSidebarLabel, zoomSignedIn } from "../lib/zoom-status";
  import ConfirmDialog from "../lib/common/ConfirmDialog.svelte";
  import ToggleSwitch from "../lib/common/ToggleSwitch.svelte";
  import { confirmDialog } from "../lib/ui";

  let fileInput: HTMLInputElement;
  async function restoreBackup(event: Event) {
    const input=event.currentTarget as HTMLInputElement; const file=input.files?.[0]; if(!file)return;
    try { const data=JSON.parse(await file.text()); const counts=`${data.rosterPeople?.length||0} roster · ${data.aliasMappings?.length||0} aliases · ${data.attendanceProfiles?.length||0} profiles`;
      if(await confirmDialog({title:t('恢复备份','Restore backup'),message:`${t('将导入以下配置：','Import this backup configuration:')} ${counts}`})){const response=await fetch('/api/backup',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(data)});if(response.ok){await loadConfig();$toastMessage=t('备份配置已导入','Backup configuration imported')}else $toastMessage=t('导入失败：','Import failed: ')+(await response.text())}
    } catch(error:any){$toastMessage=error.message} finally {input.value=''}
  }
  async function clearLocalRecords(){if(!await confirmDialog({title:t('清除本地记录','Clear local records'),message:t('确定清除所有本地记录？','Clear all local records?'),destructive:true}))return;const r=await fetch('/api/local-data',{method:'DELETE'});if(r.ok){$meeting=null;await refresh()}else $toastMessage=t('会议进行时不能清除数据。','Cannot clear while a meeting is live.')}
</script>

<section class="panel settings">
  <h2>{t("语言与界面", "Language and display")}</h2>
  <p>
    {t(
      "界面语言与导出语言分别设置。",
      "Set the interface and export language separately.",
    )}
  </p>
  <div class="language-options">
    <button
      class:chosen={$language === "zh-CN"}
      onclick={() => chooseLanguage("zh-CN")}
    >
      简体中文
    </button><button
      class:chosen={$language === "en"}
      onclick={() => chooseLanguage("en")}
    >
      English
    </button>
  </div>
  <label>
    {t("导出语言", "Export language")}
    <select bind:value={$settings.exportLanguage} onchange={saveSettings}>
      <option value="en">English</option><option value="zh">简体中文</option>
    </select>
  </label><label>
    {t("大小", "UI size")}
    <select
      value={localStorage.getItem("ui-size") || "medium"}
      onchange={(e) => localStorage.setItem("ui-size", e.currentTarget.value)}
    >
      <option value="compact">{t("紧凑", "Compact")}</option><option
        value="medium"
      >
        {t("中等", "Medium")}
      </option><option value="large">{t("大", "Large")}</option><option
        value="xlarge"
      >
        {t("特大", "X Large")}
      </option><option value="xxlarge">{t("超大", "XX Large")}</option>
    </select>
  </label><div class="settings-toggle"><ToggleSwitch label={t("默认折叠筛选器", "Collapse filters by default")} bind:checked={$settings.filterDefaultsClosed} onChange={saveSettings}/></div>
  <div class="zoom-account">
    <div>
      <b>{t("Zoom 账号", "Zoom account")}</b>
      <p>{zoomSidebarLabel($zoom, $connected, t)}</p>
    </div>
    {#if zoomSignedIn($zoom)}<button class="danger" onclick={async()=>{if(await confirmDialog({title:t('退出 Zoom 登录','Sign out of Zoom'),message:t('退出登录并移除 AttendFlow Desktop 的 Zoom 授权？考勤记录会保留在这台电脑上。','Sign out and remove AttendFlow Desktop from your Zoom account? Attendance records stay on this computer.'),destructive:true})){const r=await fetch('/api/zoom/sign-out',{method:'POST'});const d=await r.json().catch(()=>({}));if(r.ok)zoom.set(await get('/api/zoom').catch(()=>null));$toastMessage=r.ok?(d.revoked?t('已退出 Zoom 并移除授权','Signed out and access removed'):t('已退出 Zoom','Signed out of Zoom')):t('退出失败','Sign-out failed')}}}>
        {t("退出 Zoom 登录", "Sign out of Zoom")}
      </button>{:else if $zoom?.state !== "not_configured"}<button
        class="primary"
        onclick={zoomSignIn}
      >
        {t("使用 Zoom 登录", "Sign in with Zoom")}
      </button>{/if}
  </div>
  <div class="settings-actions">
    <button onclick={async()=>{try{await loadExampleDataQuiet()}catch(e:any){$toastMessage=e.message}}}>
      {t("加载示例数据", "Load example data")}
    </button><button onclick={downloadBackup}>
      {t("导出完整备份", "Export full backup")}
    </button><button onclick={() => fileInput.click()}>
      {t("恢复备份", "Restore backup")}
    </button><button
      class="danger"
      onclick={clearLocalRecords}
    >
      {t("清除本地记录", "Clear local records")}
    </button><input
      bind:this={fileInput}
      type="file"
      accept="application/json,.json"
      hidden
      onchange={restoreBackup}
    />
  </div>
  <div class="about">
    <span class="mark">A</span>
    <div>
      <b>AttendFlow Desktop</b><small>
        Zoom Companion · {t(
          "仅本地保存考勤记录",
          "Local-only attendance records",
        )}
      </small>
    </div>
  </div>
</section>
<ConfirmDialog />
