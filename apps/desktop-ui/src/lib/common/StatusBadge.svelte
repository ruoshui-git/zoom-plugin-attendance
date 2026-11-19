<script lang="ts">
  import { statusStyle } from '../status-style';
  export let label = '';
  export let tone = 'neutral';
  export let status = '';
  $: palette = status ? statusStyle(status) : tone === 'success' ? statusStyle('full_present') : tone === 'warning' ? statusStyle('late') : tone === 'danger' ? statusStyle('absent') : statusStyle('total');
</script>
<span class="badge" style={`--badge-color:${palette.color};--badge-bg:${palette.background};--badge-border:${palette.border}`}>{label}</span>
<style>.badge{display:inline-flex;align-items:center;border:1px solid var(--badge-border);border-radius:12px;padding:2px 7px;background:var(--badge-bg);color:var(--badge-color);font-size:10px;font-weight:600;line-height:1.35;white-space:nowrap}</style>
