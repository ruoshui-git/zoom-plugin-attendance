<script lang="ts">
  import { viewedMeeting, meetings, recordCounts, recordModes, selectedRecords, recordImportPreview, language } from '../lib/stores';
  import { exportRecords, deleteRecords, previewRecordImport, commitRecordImport, loadViewedRecord } from '../lib/api';
  import { t, dateTime } from '../lib/ui';
  import { filterMeetings } from '../lib/record-tools';
  import DataTable from '../lib/common/DataTable.svelte';
  import SlideOverPanel from '../lib/common/SlideOverPanel.svelte';
  import RecordDetail from './records/RecordDetail.svelte';

  let search = '';
  let modeFilter = 'all';
  let sourceFilter = 'all';
  let statusFilter = 'all';
  let detailOpen = false;
  let importInput: HTMLInputElement;
  $: filtered = filterMeetings($meetings.map((item) => ({ ...item, mode: $recordModes[item.instanceId] || item.mode })), { search, mode: modeFilter, source: sourceFilter, status: statusFilter });
  $: columns = [
    { key: 'topic', label: t('会议','Meeting'), value: (item: any) => item.topic || item.meetingNumber || '—' },
    { key: 'startedAt', label: t('开始时间','Started'), value: (item: any) => item.startedAt ? dateTime(item.startedAt) : '—', sortValue: (item: any) => Date.parse(item.startedAt || '') || 0 },
    { key: 'endedAt', label: t('结束时间','Ended'), value: (item: any) => item.endedAt ? dateTime(item.endedAt) : '—', sortValue: (item: any) => Date.parse(item.endedAt || '') || 0 },
    { key: 'mode', label: t('考勤模式','Attendance Mode'), value: (item: any) => (($recordModes[item.instanceId] || item.mode) === 'roster') ? t('花名册','Roster') : t('普通模式','Generic') },
    { key: 'attended', label: t('参与人数','# Attended'), value: (item: any) => String($recordCounts[item.instanceId] ?? '—'), sortValue: (item: any) => $recordCounts[item.instanceId] ?? -1 },
    { key: 'status', label: t('状态','Status'), value: (item: any) => item.endedAt ? (item.exportedAt ? t('已下载','Downloaded') : t('已结束','Ended')) : t('进行中','In progress') },
  ];
  const modeOptions = [['all','全部','All modes'],['generic','普通模式','Generic'],['roster','花名册','Roster']];
  const sourceOptions = [['all','全部','All sources'],['live','实时采集','Live'],['imported','导入','Imported']];
  const statusOptions = [['all','全部','All status'],['ended','已结束','Ended'],['active','进行中','Active']];
  async function openRecord(item: any) {
    detailOpen = true;
    await loadViewedRecord(item);
  }
  function selectRecords(ids: string[]) { selectedRecords.set(ids); }
  async function exportSelection() { if ($selectedRecords.length) await exportRecords($selectedRecords); else await exportRecords(); }
  async function deleteSelection() {
    if (!$selectedRecords.length) return;
    if (!confirm(t(`确定删除 ${$selectedRecords.length} 条会议记录吗？此操作无法撤销。`,`Delete ${$selectedRecords.length} meeting record(s)? This cannot be undone.`))) return;
    await deleteRecords($selectedRecords);
  }
  function filterOptions(options: string[][]) { return options.map(([value, zh, en]) => ({ value, label: t(zh,en) })); }
  $: previewPlans = $recordImportPreview?.plans || [];
</script>

<section class="records-page">
  <div class="heading"><div><h2>{t('考勤记录','Attendance Records')}</h2><p>{filtered.length} / {$meetings.length} {t('条记录','records')}</p></div><div class="actions"><button onclick={() => importInput.click()}>{t('导入','Import')}</button><input bind:this={importInput} type="file" accept=".json,application/json" hidden onchange={previewRecordImport}/><button disabled={!$meetings.length} onclick={exportSelection}>{$selectedRecords.length ? t('导出所选','Export Selected') : t('导出全部','Export All')}</button><button class="danger" disabled={!$selectedRecords.length} onclick={deleteSelection}>{t('删除所选','Delete Selected')}</button></div></div>
  <div class="filters"><label>{t('搜索日期/时间','Search date/time')}<input aria-label={t('按日期/时间或主题搜索','Search by date/time or topic')} placeholder={t('例如：2024-03、2024-03-15 或会议主题','e.g. 2024-03, 2024-03-15 or meeting topic')} bind:value={search}/></label><label>{t('考勤模式','Attendance Mode')}<select bind:value={modeFilter}>{#each filterOptions(modeOptions) as option}<option value={option.value}>{option.label}</option>{/each}</select></label><label>{t('来源','Source')}<select bind:value={sourceFilter}>{#each filterOptions(sourceOptions) as option}<option value={option.value}>{option.label}</option>{/each}</select></label><label>{t('状态','Status')}<select bind:value={statusFilter}>{#each filterOptions(statusOptions) as option}<option value={option.value}>{option.label}</option>{/each}</select></label></div>
  <DataTable items={filtered} {columns} selectedIds={$selectedRecords} rowId={(item) => String(item.instanceId)} emptyLabel={t('没有符合条件的会议记录','No matching meeting records')} selectAllLabel={t('全选','Select all')} onSelectionChange={selectRecords} onRowClick={openRecord}/>
</section>

<SlideOverPanel open={detailOpen && !!$viewedMeeting} title={t('考勤记录详情','Attendance Record Detail')} closeLabel={t('关闭','Close')} width={960} onClose={() => detailOpen = false}>
  {#if $viewedMeeting}<RecordDetail />{/if}
</SlideOverPanel>

<SlideOverPanel open={!!$recordImportPreview} title={t('会议记录导入预览','Import Attendance Records')} closeLabel={t('关闭','Close')} onClose={() => recordImportPreview.set(null)}>
  {#if $recordImportPreview}<p class="intro">{t('检查每条记录的处理结果，再确认导入。','Review how each record will be handled before confirming the import.')}</p><div class="summary"><b>{previewPlans.length}</b> {t('条导入计划','planned record(s)')}<span>{previewPlans.filter((plan: any) => plan.action === 'reject').length} {t('条被拒绝','rejected')}</span></div><div class="preview-wrap"><table><thead><tr><th>{t('操作','Action')}</th><th>{t('会议','Meeting')}</th><th>{t('说明','Reason')}</th></tr></thead><tbody>{#each previewPlans as plan}<tr><td><span class:reject={plan.action==='reject'} class="action-pill">{plan.action === 'add' ? t('新增','Add') : plan.action === 'update' ? t('更新','Update') : t('拒绝','Reject')}</span></td><td>{plan.value?.meeting?.topic || plan.value?.meeting?.meetingNumber || plan.value?.record?.meeting?.topic || '—'}</td><td>{plan.reason}</td></tr>{:else}<tr><td colspan="3" class="empty">{t('没有可预览的记录','No records to preview')}</td></tr>{/each}</tbody></table></div><div class="panel-actions"><button class="primary" disabled={previewPlans.some((plan: any) => plan.action === 'reject')} onclick={commitRecordImport}>{t('确认导入','Confirm Import')}</button><button onclick={() => recordImportPreview.set(null)}>{t('取消','Cancel')}</button></div>{/if}
</SlideOverPanel>
<style>
 .records-page{display:grid;gap:12px}.heading{display:flex;align-items:center;justify-content:space-between;gap:12px}.heading h2{font-size:16px;margin:0}.heading p{font-size:10px;color:var(--muted);margin:4px 0 0}.actions{display:flex;gap:6px;flex-wrap:wrap}.actions button{min-height:29px;padding:5px 9px;font-size:10px}.filters{display:flex;align-items:end;gap:9px;flex-wrap:wrap;background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:9px}.filters label{display:grid;gap:4px;color:var(--muted);font-size:9px;flex:1;min-width:130px}.filters input,.filters select{min-height:30px!important;padding:5px 8px!important;font-size:11px!important}.intro{font-size:11px;color:var(--muted);margin:0 0 12px}.summary{font-size:11px;padding:10px;background:var(--surface-soft);border:1px solid var(--line);border-radius:7px;margin-bottom:12px}.summary span{float:right;color:var(--danger)}.preview-wrap{overflow:auto;border:1px solid var(--line);border-radius:8px}.preview-wrap table{width:100%;border-collapse:collapse;font-size:11px}.preview-wrap th,.preview-wrap td{padding:8px;border-bottom:1px solid var(--line);text-align:left;vertical-align:top}.preview-wrap th{position:sticky;top:0;background:#f5f6fa;font-size:9px;text-transform:uppercase;color:var(--muted)}.action-pill{font-size:10px;color:var(--success);background:#eaf8f0;border-radius:14px;padding:3px 7px}.action-pill.reject{color:var(--danger);background:#fff0f0}.empty{text-align:center;color:var(--muted);padding:22px!important}.panel-actions{display:flex;justify-content:flex-end;gap:7px;margin-top:14px}.panel-actions button{font-size:11px;min-height:30px}
</style>
