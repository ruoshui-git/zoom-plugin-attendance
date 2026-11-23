<script lang="ts">
  import { viewedMeeting, meetings, recordCounts, recordModes, selectedRecords, recordImportPreview } from '../lib/stores';
  import { exportRecords, deleteRecords, previewRecordImport, commitRecordImport, loadViewedRecord } from '../lib/api';
  import { t, dateTime, confirmDialog } from '../lib/ui';
  import { toast } from '../lib/toast';
  import { runBusy, busyKeys } from '../lib/busy';
  import { filterMeetings } from '../lib/record-tools';
  import DataTable from '../lib/common/DataTable.svelte';
  import SlideOverPanel from '../lib/common/SlideOverPanel.svelte';
  import RecordDetail from './records/RecordDetail.svelte';

  let search = '';
  let detailOpen = false;
  let importInput: HTMLInputElement;
  let bulkMode = false;
  let pasteOpen = false;
  let importText = '';
  $: filtered = filterMeetings($meetings, { search });
  $: columns = [
    { key: 'topic', label: t('会议','Meeting'), value: (item: any) => item.topic || item.meetingNumber || '—' },
    { key: 'startedAt', label: t('开始时间','Started'), value: (item: any) => item.startedAt ? dateTime(item.startedAt) : '—', sortValue: (item: any) => Date.parse(item.startedAt || '') || 0 },
    { key: 'endedAt', label: t('结束时间','Ended'), value: (item: any) => item.endedAt ? dateTime(item.endedAt) : '—', sortValue: (item: any) => Date.parse(item.endedAt || '') || 0 },
    { key: 'mode', label: t('考勤模式','Attendance Mode'), value: (item: any) => (($recordModes[item.instanceId] || item.mode) === 'roster') ? t('花名册','Roster') : t('普通模式','Generic') },
    { key: 'attended', label: t('参与人数','# Attended'), value: (item: any) => String($recordCounts[item.instanceId] ?? '—'), sortValue: (item: any) => $recordCounts[item.instanceId] ?? -1 },
    { key: 'status', label: t('状态','Status'), value: (item: any) => item.endedAt ? (item.exportedAt ? t('已下载','Downloaded') : t('已结束','Ended')) : t('进行中','In progress') },
  ];
  async function openRecord(item: any) {
    detailOpen = true;
    await loadViewedRecord(item);
  }
  function selectRecords(ids: string[]) { selectedRecords.set(ids); }
  async function performExport(ids: string[] = [], selected = false) { await runBusy('records-export', async () => { try { await exportRecords(ids); toast.success(t(selected ? '已导出选中项。' : '考勤记录已导出。', selected ? 'Selected items exported.' : 'Attendance records exported.')); } catch { toast.error(t('导出失败。', 'Export failed.')); } }); }
  async function exportSelection() { await performExport($selectedRecords, true); }
  async function previewImport(input: Event | unknown) { await runBusy('records-preview', async () => { try { await previewRecordImport(input); if (!(input instanceof Event)) pasteOpen = false; } catch (error: any) { toast.error(`${t('预览失败：', 'Preview failed: ')}${error.message}`); } }); }
  function previewFile(event: Event) { void previewImport(event); }
  async function confirmImport() { await runBusy('records-import', async () => { try { await commitRecordImport(); } catch (error: any) { toast.error(`${t('导入失败：', 'Import failed: ')}${error.message}`); } }); }
  async function deleteSelection() {
    if (!$selectedRecords.length) return;
    const selected = $meetings.filter((item) => $selectedRecords.includes(String(item.instanceId)));
    const ended = selected.filter((item) => !!item.endedAt);
    const skipped = selected.length - ended.length;
    if (!ended.length) return;
    if (!(await confirmDialog({ title: t('删除考勤记录','Delete Attendance Records'), message: t(`确定删除 ${ended.length} 条会议记录吗？此操作无法撤销。`,`Delete ${ended.length} meeting record(s)? This cannot be undone.`), destructive: true }))) return;
    await runBusy('records-delete', async () => { try {
      await deleteRecords(ended.map((item) => String(item.instanceId)));
      toast.success(t(`成功删除 ${ended.length} 份考勤记录。${skipped ? ' 已跳过正在进行的会议。' : ''}`, `Successfully deleted ${ended.length} attendance record(s).${skipped ? ' The in-progress meeting was skipped.' : ''}`));
      selectedRecords.set([]); bulkMode = false;
    } catch (error: any) { toast.error(`${t('批量删除失败：', 'Bulk delete failed: ')}${error.message}`); } });
  }
  async function previewPaste() {
    try { const parsed = JSON.parse(importText); await previewImport(Array.isArray(parsed) ? parsed : parsed.records || parsed.sessions || [parsed]); }
    catch (error: any) { toast.error(`${t('预览失败：', 'Preview failed: ')}${error.message}`); }
  }
  $: previewPlans = $recordImportPreview?.plans || [];
</script>

<section class="records-page">
  <div class="heading"><div><h2>{t('考勤记录','Attendance Records')}</h2><p>{filtered.length} / {$meetings.length} {t('条记录','records')}</p></div><div class="actions">{#if !bulkMode}<button disabled={$busyKeys.has('records-preview')} onclick={() => importInput.click()}>{$busyKeys.has('records-preview') ? t('预览中…','Previewing…') : t('导入','Import')}</button><button disabled={!$meetings.length || $busyKeys.has('records-export')} onclick={() => performExport()}>{$busyKeys.has('records-export') ? t('导出中…','Exporting…') : t('导出','Export')}</button><button onclick={() => pasteOpen = true}>{t('粘贴 JSON','Paste JSON')}</button><button onclick={() => bulkMode = true}>{t('批量管理','Bulk Select')}</button>{:else}<span>{t(`已选择 ${$selectedRecords.length} 项`, `${$selectedRecords.length} selected`)}</span><button disabled={!$selectedRecords.length || $busyKeys.has('records-export')} onclick={exportSelection}>{$busyKeys.has('records-export') ? t('导出中…','Exporting…') : t('导出选中','Export Selected')}</button><button class="danger" disabled={$busyKeys.has('records-delete') || !$selectedRecords.some((id) => $meetings.some((m) => String(m.instanceId) === id && !!m.endedAt))} onclick={deleteSelection}>{$busyKeys.has('records-delete') ? t('删除中…','Deleting…') : t('删除选中','Delete Selected')}</button><button onclick={() => { selectedRecords.set([]); bulkMode = false; }}>{t('取消','Cancel')}</button>{/if}<input bind:this={importInput} type="file" accept=".json,application/json" multiple hidden onchange={previewFile}/></div></div>
  <div class="filters"><label>{t('搜索日期/时间','Search date/time')}<input aria-label={t('按日期/时间或主题搜索','Search by date/time or topic')} placeholder={t('例如：2024-03、2024-03-15 或会议主题','e.g. 2024-03, 2024-03-15 or meeting topic')} bind:value={search}/></label></div>
  <DataTable items={filtered} {columns} bulkMode={bulkMode} selectedIds={$selectedRecords} rowId={(item) => String(item.instanceId)} emptyLabel={search ? t('未搜到相应考勤记录。','No attendance records found.') : t('暂无考勤记录。','No attendance records yet.')} selectAllLabel={t('全选','Select all')} onSelectionChange={selectRecords} onRowClick={openRecord}/>
</section>

<SlideOverPanel open={detailOpen && !!$viewedMeeting} title={t('考勤记录详情','Attendance Record Detail')} closeLabel={t('关闭','Close')} width={960} onClose={() => detailOpen = false}>
  {#if $viewedMeeting}<RecordDetail />{/if}
</SlideOverPanel>
<SlideOverPanel open={pasteOpen} title={t('粘贴 JSON 导入','Paste JSON Import')} closeLabel={t('关闭','Close')} onClose={() => pasteOpen = false}><textarea bind:value={importText} placeholder={t('粘贴 JSON 记录…','Paste JSON records…')}></textarea><div class="panel-actions"><button class="primary" disabled={$busyKeys.has('records-preview')} onclick={previewPaste}>{$busyKeys.has('records-preview') ? t('预览中…','Previewing…') : t('生成预览','Preview')}</button><button onclick={() => pasteOpen = false}>{t('取消','Cancel')}</button></div></SlideOverPanel>

<SlideOverPanel open={!!$recordImportPreview} title={t('会议记录导入预览','Import Attendance Records')} closeLabel={t('关闭','Close')} onClose={() => recordImportPreview.set(null)}>
  {#if $recordImportPreview}<p class="intro">{t('检查每条记录的处理结果，再确认导入。','Review how each record will be handled before confirming the import.')}</p><div class="summary">{t('当前数据','Current')}: <b>{$meetings.length}</b>　{t('导入后','After Import')}: <b>{$meetings.length + previewPlans.filter((plan: any) => plan.action === 'add').length}</b><span>{previewPlans.filter((plan: any) => plan.action === 'reject').length} {t('条被拒绝','rejected')}</span></div><div class="preview-wrap"><table><thead><tr><th>{t('操作','Action')}</th><th>{t('会议','Meeting')}</th><th>{t('说明','Reason')}</th></tr></thead><tbody>{#each previewPlans as plan}<tr><td><span class:reject={plan.action==='reject'} class="action-pill">{plan.action === 'add' ? t('新增','Add') : plan.action === 'update' ? t('更新','Update') : t('拒绝','Reject')}</span></td><td>{plan.value?.meeting?.topic || plan.value?.meeting?.meetingNumber || plan.value?.record?.meeting?.topic || '—'}</td><td>{plan.reason}</td></tr>{:else}<tr><td colspan="3" class="empty">{t('没有可预览的记录','No records to preview')}</td></tr>{/each}</tbody></table></div><div class="panel-actions"><button class="primary" disabled={$busyKeys.has('records-import') || previewPlans.some((plan: any) => plan.action === 'reject')} onclick={confirmImport}>{$busyKeys.has('records-import') ? t('导入中…','Importing…') : t('确认导入','Confirm Import')}</button><button onclick={() => recordImportPreview.set(null)}>{t('取消','Cancel')}</button></div>{/if}
</SlideOverPanel>
<style>
 .records-page{display:grid;gap:12px}.heading{display:flex;align-items:center;justify-content:space-between;gap:12px}.heading h2{font-size:16px;margin:0}.heading p{font-size:10px;color:var(--muted);margin:4px 0 0}.actions{display:flex;gap:6px;flex-wrap:wrap}.actions button{min-height:29px;padding:5px 9px;font-size:10px}.filters{display:flex;align-items:end;gap:9px;flex-wrap:wrap;background:var(--surface);border:1px solid var(--line);border-radius:8px;padding:9px}.filters label{display:grid;gap:4px;color:var(--muted);font-size:9px;flex:1;min-width:130px}.filters input{min-height:30px!important;padding:5px 8px!important;font-size:11px!important}.intro{font-size:11px;color:var(--muted);margin:0 0 12px}.summary{font-size:11px;padding:10px;background:var(--surface-soft);border:1px solid var(--line);border-radius:7px;margin-bottom:12px}.summary span{float:right;color:var(--danger)}.preview-wrap{overflow:auto;border:1px solid var(--line);border-radius:8px}.preview-wrap table{width:100%;border-collapse:collapse;font-size:11px}.preview-wrap th,.preview-wrap td{padding:8px;border-bottom:1px solid var(--line);text-align:left;vertical-align:top}.preview-wrap th{position:sticky;top:0;background:#f5f6fa;font-size:9px;text-transform:uppercase;color:var(--muted)}.action-pill{font-size:10px;color:var(--success);background:#eaf8f0;border-radius:14px;padding:3px 7px}.action-pill.reject{color:var(--danger);background:#fff0f0}.empty{text-align:center;color:var(--muted);padding:22px!important}.panel-actions{display:flex;justify-content:flex-end;gap:7px;margin-top:14px}.panel-actions button{font-size:11px;min-height:30px}
</style>
