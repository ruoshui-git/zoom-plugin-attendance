<script lang="ts">
  import { page, mode } from "./stores";
  import { t } from "./ui";

  export let matchingCount = 0;
  const nav = [
    ["live", "◉", "Live", "出勤"],
    ["records", "▤", "Records", "考勤记录"],
    ["configuration", "⌘", "Configuration", "考勤配置"],
    ["settings", "⚙", "Settings", "其他设置"],
  ];
</script>

<nav>
  {#each nav as [key, icon, en, zh]}
    <button class:active={$page === key} onclick={() => page.set(key)}>
      <span>{icon}</span>{t(zh, en)}
      {#if key === "live" && $mode === "roster" && matchingCount > 0}
        <small class="nav-badge">{matchingCount}</small>
      {/if}
    </button>
  {/each}
</nav>
