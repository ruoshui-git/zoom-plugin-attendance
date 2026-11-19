<script lang="ts">export let label = ""; export let tone = "neutral";</script>
<span class="badge" class:success={tone==='success'} class:warning={tone==='warning'} class:danger={tone==='danger'}>{label}</span>
<style>.badge{display:inline-flex;align-items:center;border-radius:12px;padding:2px 7px;background:#edf1f5;color:#596273;font-size:10px;line-height:1.35;white-space:nowrap}.badge.success{background:#eaf7f0;color:#228353}.badge.warning{background:#fff3e8;color:#ba6a28}.badge.danger{background:#fff0f0;color:#b42332}</style>
