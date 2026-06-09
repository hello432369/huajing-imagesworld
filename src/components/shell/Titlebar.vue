<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window"

const appWindow = getCurrentWindow()
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="traffic-lights">
      <button class="traffic-dot close" @click="appWindow.close()" title="关闭" />
      <button class="traffic-dot minimize" @click="appWindow.minimize()" title="最小化" />
      <button class="traffic-dot zoom" @click="appWindow.toggleMaximize()" title="缩放" />
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  height: var(--titlebar-height);
  background: var(--bg-deepest);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  padding: 0 12px;
  user-select: none;
}

.traffic-lights {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 0 4px;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.traffic-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  cursor: pointer;
  transition: opacity 0.15s;
  border: none;
  padding: 0;
  appearance: none;
  -webkit-appearance: none;
}

.traffic-dot:hover {
  opacity: 0.7;
}

.traffic-dot:active {
  opacity: 0.5;
}

.close { background: #ff5f57; }
.minimize { background: #febc2e; }
.zoom { background: #28c840; }
</style>
