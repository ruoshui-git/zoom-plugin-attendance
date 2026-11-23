<script lang="ts">
  export let changes: any[] = [];
  export let t: (zh: string, en: string) => string;
  const action = (c: any) => String(c.action || c.status || "").toLowerCase();
  $: total = changes.length;
  $: adds = changes.filter((c) => ["add", "added", "create", "created"].includes(action(c))).length;
  $: updates = changes.filter((c) => ["update", "updated"].includes(action(c))).length;
  $: deletes = changes.filter((c) => ["delete", "deleted", "remove", "removed"].includes(action(c))).length;
  $: rejected = changes.filter((c) => ["reject", "rejected"].includes(action(c)) || c.rejected === true).length;
  $: warnings = changes.reduce((n, c) => n + (Array.isArray(c.warnings) ? c.warnings.length : c.warning ? 1 : 0), 0);
  $: applicable = total - rejected;
</script>
<div class="summary-grid">
  {#each [["总行数", "Total Rows", total], ["新增", "Adds", adds], ["更新", "Updates", updates], ["删除", "Deletes", deletes], ["拒绝", "Rejected", rejected], ["预警", "Warnings", warnings], ["可导入", "Applicable", applicable]] as [zh, en, count]}
    <div class="summary-card"><strong>{t(zh, en)}</strong><span>{count}</span></div>
  {/each}
</div>
<style>
  .summary-grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(100px,1fr)); gap:8px; margin:12px 0; }
  .summary-card { display:flex; flex-direction:column; gap:4px; padding:10px; border:1px solid var(--line); border-radius:8px; background:var(--surface); }
  .summary-card strong { font-size:12px; color:#596273; } .summary-card span { font-weight:700; }
</style>
