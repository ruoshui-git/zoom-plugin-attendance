import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
export default defineConfig({plugins:[svelte()],resolve:{conditions:['browser']},server:{proxy:{'/api':'http://127.0.0.1:47812','/bridge':{target:'ws://127.0.0.1:47812',ws:true}}}});
