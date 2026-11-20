<script lang="ts">
  import { settings } from "../stores";
  import { filterRowsByTag } from "../record-tools";
  import { applyFilterDragEnd, sortGroupsByProfilePatterns, sortAttendanceRows, rowMatchesSearch } from "../attendance-filters";
  import { groupKey } from "../group-label";
  import { DragDropProvider, DragOverlay, PointerSensor } from "@dnd-kit/svelte";
  import { PointerActivationConstraints } from "@dnd-kit/dom";
  import { createSortable } from "@dnd-kit/svelte/sortable";
  import { move } from "@dnd-kit/helpers";
  export let rows: any[] = [];
  export let roster: any[] = [];
  export let configGroups: string[] = [];
  export let profile: any = null;
  export let mode = "generic";
  export let language = "en";
  export let initiallyOpen = true;
  export let activeFilter = "";
  export let ascending = true;
  export let onRows: (rows: any[]) => void = () => {};
  export let onActiveFilterChange: (filter: string) => void = () => {};
  let filterOpen = initiallyOpen, appliedFilterDefault = !initiallyOpen;
  let sectionOrder = ["expectation", "group"], expectationOrder = ["expected", "unexpected"], groupOrder: string[] = [];
  let groupNames: string[] = [], lastProfileOrderKey = "", expectations = ["expected", "unexpected"], selectedGroups: string[] = [];
  let expectationOpen = true, groupOpen = true, search = "";
  const sensors = [PointerSensor.configure({ activationConstraints: [new PointerActivationConstraints.Distance({ value: 5 })] })];
  const t = (zh: string, en: string) => language === "zh-CN" ? zh : en;
  $: rosterMode = mode === "roster" && roster.length > 0;
  $: groupNames = [...new Set<string>([...configGroups.map(groupKey), ...roster.map((p: any) => groupKey(p.group))])];
  $: profileGroupOrder = profile?.groupSortOrder?.length ? sortGroupsByProfilePatterns(groupNames, profile.groupSortOrder) : groupNames.slice().sort((a,b) => a.localeCompare(b, "zh-CN"));
  $: profileOrderKey = profileGroupOrder.join("\u0000");
  $: if (profileOrderKey !== lastProfileOrderKey) { lastProfileOrderKey = profileOrderKey; groupOrder = [...profileGroupOrder]; }
  $: if (!groupOrder.length || groupOrder.some((g) => !groupNames.includes(g))) { groupOrder = [...groupOrder.filter((g) => groupNames.includes(g)), ...profileGroupOrder.filter((g) => !groupOrder.includes(g))]; selectedGroups = [...selectedGroups.filter((g) => groupNames.includes(g)), ...groupNames.filter((g) => !selectedGroups.includes(g))]; }
  $: if (groupNames.length && !selectedGroups.length) selectedGroups = [...groupNames];
  $: if ($settings.filterDefaultsClosed !== appliedFilterDefault) { appliedFilterDefault = $settings.filterDefaultsClosed; filterOpen = !$settings.filterDefaultsClosed; }
  $: sortedRows = sortAttendanceRows(rows, { sectionOrder: rosterMode ? sectionOrder : [], groups: groupOrder, expectations: expectationOrder, selectedGroups, selectedExpectations: expectations, ascending });
  $: filteredRows = filterRowsByTag(sortedRows, activeFilter).filter((r: any) => (!rosterMode || selectedGroups.includes(groupKey(r.group))) && (!rosterMode || expectations.includes(r.isExpected === false ? "unexpected" : "expected")) && rowMatchesSearch(r, search));
  $: onRows(filteredRows);
  function handleDragEnd(event: any) {
    const visible = sectionOrder.filter((section) => section !== "expectation" || rows.some((row: any) => row.isExpected === false));
    const next = applyFilterDragEnd({ sections: sectionOrder, groups: groupOrder, expectations: expectationOrder }, event, move as any, visible);
    sectionOrder = next.sections; groupOrder = next.groups; expectationOrder = next.expectations;
  }
  function reset() { activeFilter = ""; onActiveFilterChange(""); expectations = ["expected", "unexpected"]; expectationOrder = ["expected", "unexpected"]; selectedGroups = [...groupNames]; groupOrder = [...profileGroupOrder]; sectionOrder = ["expectation", "group"]; search = ""; ascending = true; }
</script>

<div class="result-bar" aria-live="polite"><span class="result-count">{t('当前显示', 'Showing')} <strong>{filteredRows.length}</strong> / {rows.length} {t('人', 'participants')}</span></div>
<div class="filter-panel" class:expanded={filterOpen}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
  <div class="filter-section-bar" role="button" aria-expanded={filterOpen} tabindex="0" onclick={() => filterOpen = !filterOpen} onkeydown={(event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); filterOpen = !filterOpen; } }}>
    <span class="filter-toggle-btn"><span class="filter-toggle-icon">{filterOpen ? '▼' : '▶'}</span><span>{t('筛选与排序', 'Filters & Sort')} · {t('分组', 'groups')} {rosterMode ? selectedGroups.length : 0}/{rosterMode ? groupNames.length : 0}</span></span>
    <div class="filter-section-actions" role="group" onclick={(event) => event.stopPropagation()}><button class="filter-section-btn" aria-label={t('全部重置','Reset All')} onclick={reset}>↺</button><button class="filter-section-btn" aria-label={t('全部展开','Expand All')} onclick={() => { expectationOpen = true; groupOpen = true; }}>⊞</button><button class="filter-section-btn" aria-label={t('全部折叠','Collapse All')} onclick={() => { expectationOpen = false; groupOpen = false; }}>⊟</button></div>
  </div>
  {#if filterOpen}
    <div class="filter-panel-body"><div class="filter-name-search"><div class="filter-name-input-wrap"><input class="filter-name-input" aria-label={t('搜索姓名','Search names')} placeholder={t('姓名/拼音…','Name / pinyin…')} bind:value={search}/>{#if search}<button class="filter-name-clear" aria-label={t('清除','Clear')} onclick={() => search = ''}>✕</button>{/if}</div><button class="sort-dir-btn" aria-label={t('切换姓名排序方向','Toggle name sort direction')} onclick={() => ascending = !ascending}>{ascending ? 'A→Z' : 'Z→A'}</button></div>
    {#if rosterMode}      <DragDropProvider {sensors} onDragEnd={handleDragEnd}>{#each sectionOrder.filter((s) => s !== 'expectation' || rows.some((r: any) => r.isExpected === false)) as section, si (section)}
        {@const sortable = createSortable({id: section, index: si, type: 'filter-section', accept: 'filter-section'})}
        <div class="section-wrapper" data-dragging={sortable.isDragging ? 'true' : undefined} data-over={sortable.isDropTarget ? 'true' : undefined} {@attach sortable.attach}>
        <section class="filter-group" data-section={section}>
          <div class="filter-group-header"><button class="drag-handle" aria-label={t('拖动排序','Drag to reorder')} {@attach sortable.attachHandle}>⋮⋮</button><button class="section-toggle" onclick={() => section === 'group' ? groupOpen = !groupOpen : expectationOpen = !expectationOpen}>{(section === 'group' ? groupOpen : expectationOpen) ? '▼' : '▶'} {section === 'group' ? t('分组筛选','Groups') : t('范围筛选','Roster Scope')}</button><span class="section-count">{section === 'group' ? `${selectedGroups.length}/${groupNames.length}` : `${expectations.length}/2`}</span></div>
          {#if section === 'group' ? groupOpen : expectationOpen}<div class="chip-filters">{#if section === 'group'}{#each groupOrder as group, gi (group)}{@const chip = createSortable({id: group, index: gi, type: 'filter-group', accept: 'filter-group'})}<button class="filter-chip" class:active={selectedGroups.includes(group)} data-dragging={chip.isDragging} {@attach chip.attach} {@attach chip.attachHandle} onclick={() => selectedGroups = selectedGroups.includes(group) ? selectedGroups.filter((g) => g !== group) : [...selectedGroups, group]}>{group}</button>{/each}{:else}{#each expectationOrder as expectation, ei (expectation)}{@const chip = createSortable({id: expectation, index: ei, type: 'filter-expectation', accept: 'filter-expectation'})}<button class="filter-chip" class:active={expectations.includes(expectation)} data-dragging={chip.isDragging} {@attach chip.attach} {@attach chip.attachHandle} onclick={() => expectations = expectations.includes(expectation) ? expectations.filter((g) => g !== expectation) : [...expectations, expectation]}>{expectation === 'expected' ? t('应到成员','Expected Roster') : t('范围外成员','Unexpected Roster')}</button>{/each}{/if}</div>{/if}
        </section>
        </div>
      {/each}<DragOverlay>{#snippet children(source: any)}<div class="section-overlay-card"><span class="section-overlay-handle" aria-hidden="true">⋮⋮</span><span class="section-overlay-expand" aria-hidden="true">▶</span><span class="section-overlay-title">{source.id === 'group' ? t('分组筛选','Groups') : t('范围筛选','Roster Scope')}</span></div>{/snippet}</DragOverlay></DragDropProvider>
    {/if}</div>
  {/if}
</div>

<style>
  .result-bar{display:flex;align-items:center;gap:8px;margin:6px 0}.result-count{font-size:13px;color:var(--muted);font-weight:500}.result-count strong{color:var(--accent);font-weight:650}
  .filter-panel{margin:5px 0 8px;border:1px solid var(--line);border-radius:var(--radius-md);overflow:hidden;background:var(--surface)}.filter-panel-body{background:var(--surface)}
  .filter-section-bar{display:flex;align-items:center;justify-content:space-between;padding:6px 9px;background:var(--surface-soft);cursor:pointer}.filter-panel.expanded .filter-section-bar{border-bottom:1px solid var(--line)}.filter-section-bar:hover{background:var(--accent-soft)}.filter-toggle-btn{display:flex;align-items:center;gap:7px;font-size:12px;font-weight:650;color:var(--text)}.filter-toggle-icon{width:10px;text-align:center;font-size:9px;color:var(--muted)}.filter-section-actions{display:flex;gap:3px}.filter-section-btn,.sort-dir-btn{min-width:27px;height:27px;min-height:27px;padding:0 7px;border:1px solid var(--line);border-radius:var(--radius-sm);background:var(--surface);color:var(--text);font-size:12px}.filter-section-btn:hover,.sort-dir-btn:hover{border-color:var(--accent);color:var(--accent)}
  .filter-name-search{display:flex;align-items:center;gap:6px;padding:7px 9px;border-bottom:1px solid var(--line);background:var(--surface-soft)}.filter-name-input-wrap{position:relative;flex:1}.filter-name-input{width:100%;padding-right:28px!important}.filter-name-clear{position:absolute;right:4px;top:50%;transform:translateY(-50%);min-height:22px;padding:2px 4px;border:0;background:transparent;color:var(--muted)}
  .section-wrapper{position:relative}.section-wrapper[data-dragging="true"]{opacity:.35}.section-wrapper[data-over="true"]::before{content:"";position:absolute;inset:0 0 auto;height:2px;background:var(--accent);pointer-events:none;z-index:2}.filter-group{position:relative;border-bottom:1px solid var(--line);padding:0 9px 7px}.section-wrapper:last-of-type .filter-group{border-bottom:0}.filter-group-header{display:flex;align-items:center;gap:6px;min-height:32px}.drag-handle{min-height:23px;padding:0 4px;border:0;background:transparent;color:var(--muted);cursor:grab;touch-action:none}.drag-handle:active{cursor:grabbing}.section-toggle{min-height:25px;padding:2px 4px;border:0;background:transparent;font-size:11px;font-weight:650}.section-count{margin-left:auto;color:var(--muted);font-size:10px}.chip-filters{display:flex;gap:6px;flex-wrap:wrap;padding:3px 4px}.filter-chip{min-height:27px;padding:4px 10px;border:1px solid var(--line);border-radius:999px;background:var(--surface);color:var(--text-secondary,#3d3d56);font-size:11px;touch-action:none;cursor:grab}.filter-chip.active{border-color:color-mix(in srgb,var(--accent) 42%,white);background:var(--accent-soft);color:var(--accent)}.filter-chip[data-dragging="true"]{opacity:.35}.section-overlay-card{display:flex;align-items:center;gap:6px;min-height:30px;padding:5px 8px;border:1px solid var(--line);border-radius:0;background:var(--surface);color:var(--text);font-size:12px;font-weight:600;box-shadow:var(--shadow-card)}.section-overlay-handle{font-size:11px;letter-spacing:-1px;color:var(--muted);line-height:1}.section-overlay-expand{width:10px;text-align:center;font-size:9px;color:var(--muted)}
</style>
