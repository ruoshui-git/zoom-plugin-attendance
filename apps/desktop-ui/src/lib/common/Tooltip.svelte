<script lang="ts">export let text = "";</script>
<span class="tip" aria-label={text}><slot /><span class="bubble" role="tooltip">{text}</span></span>
<style>.tip{position:relative;display:inline-flex}.bubble{position:absolute;z-index:10;left:50%;bottom:calc(100% + 6px);transform:translateX(-50%);display:none;max-width:240px;width:max-content;padding:6px 8px;border-radius:5px;background:#202a39;color:#fff;font-size:10px;white-space:normal}.tip:hover .bubble,.tip:focus .bubble{display:block}</style>
