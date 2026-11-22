<script lang="ts">
  export let items: any[] = [];
  export let columns: { key: string; label: string; value?: (row: any) => any; sortValue?: (row: any) => any }[] = [];
  export let selectedIds: string[] = [];
  export let rowId: (row: any) => string = (row) => String(row.id ?? row.instanceId);
  export let emptyLabel = '';
  export let selectAllLabel = '';
  export let onSelectionChange: (ids: string[]) => void = () => {};
  export let onRowClick: (row: any) => void = () => {};
  let sortKey = '';
  let sortDirection: 'asc' | 'desc' = 'asc';
  $: ordered = sortKey ? [...items].sort((a, b) => {
    const col = columns.find((item) => item.key === sortKey);
    const av = col?.sortValue ? col.sortValue(a) : col?.value ? col.value(a) : a[sortKey];
    const bv = col?.sortValue ? col.sortValue(b) : col?.value ? col.value(b) : b[sortKey];
    const result = typeof av === 'number' && typeof bv === 'number' ? av - bv : String(av ?? '').localeCompare(String(bv ?? ''), undefined, { numeric: true });
    return result * (sortDirection === 'asc' ? 1 : -1);
  }) : items;
  function toggleSort(key: string) { sortDirection = sortKey === key && sortDirection === 'asc' ? 'desc' : 'asc'; sortKey = key; }
  function selectAll(checked: boolean) { onSelectionChange(checked ? items.map(rowId) : []); }
  function selectOne(id: string, checked: boolean) { onSelectionChange(checked ? [...selectedIds, id] : selectedIds.filter((value) => value !== id)); }
</script>
<div class="data-table-wrap"><table class="data-table"><thead><tr><th class="select"><input type="checkbox" aria-label={selectAllLabel} checked={items.length > 0 && items.every((row) => selectedIds.includes(rowId(row)))} onchange={(event) => selectAll(event.currentTarget.checked)} /></th>{#each columns as column}<th><button class="sort" onclick={() => toggleSort(column.key)}>{column.label}{#if sortKey === column.key}<span>{sortDirection === 'asc' ? '↑' : '↓'}</span>{/if}</button></th>{/each}</tr></thead><tbody>{#each ordered as row (rowId(row))}<tr><td class="select"><input type="checkbox" aria-label={rowId(row)} checked={selectedIds.includes(rowId(row))} onchange={(event) => selectOne(rowId(row), event.currentTarget.checked)} onclick={(event) => event.stopPropagation()} /></td>{#each columns as column}<td><button class="cell" class:topic-cell={column.key === 'topic'} title={column.key === 'topic' ? (row.topic || row.meetingNumber || '—') : undefined} onclick={() => onRowClick(row)}>{column.value ? column.value(row) : row[column.key]}</button></td>{/each}</tr>{:else}<tr><td colspan={columns.length + 1} class="empty">{emptyLabel}</td></tr>{/each}</tbody></table></div>
<style>
  .data-table-wrap{overflow:auto;border:1px solid var(--line);border-radius:8px;background:var(--surface)}.data-table{width:100%;min-width:760px;border-collapse:collapse;font-size:11px}.data-table th{position:sticky;top:0;z-index:1;background:#f5f6fa;color:#525d70;text-align:left;font-size:10px;letter-spacing:.35px;text-transform:uppercase;font-weight:700}.data-table th,.data-table td{padding:7px 10px;border-bottom:1px solid var(--line);vertical-align:middle}.data-table tr:last-child td{border-bottom:0}.data-table tbody tr:hover{background:#f8faff}.select{width:34px;text-align:center}.sort,.cell{border:0;background:transparent;padding:0;min-height:24px;border-radius:0;color:inherit;font-size:inherit;text-align:left}.sort{font-weight:inherit;letter-spacing:inherit;text-transform:inherit}.sort:hover,.cell:hover{background:transparent;color:var(--accent)}.cell{width:100%;justify-content:flex-start}.cell.topic-cell{display:block;max-width:360px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.empty{text-align:center;color:var(--muted);padding:28px!important}
</style>
