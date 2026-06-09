<script setup lang="ts">
import AppShell from "./components/shell/AppShell.vue"
</script>

<template>
  <AppShell />
</template>

<style>
*,
*::before,
*::after {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

:root {
  --bg-deepest: #0d0d0d;
  --bg-main: #131313;
  --bg-surface: #1c1c1c;
  --bg-hover: #232323;
  --bg-active: #2a2a2a;
  --border: #252525;
  --border-light: #2e2e2e;
  --text-primary: #e0e0e0;
  --text-secondary: #8b8b8b;
  --text-muted: #555555;
  --accent: #5b9bd5;
  --accent-hover: #4a86c8;
  --accent-muted: rgba(91, 155, 213, 0.12);
  --accent-warm: #e8a87c;
  --danger: #d45a5a;
  --success: #5ab87a;
  --scrollbar-thumb: #2e2e2e;
  --scrollbar-track: transparent;
  --sidebar-width: 220px;
  --inspector-width: 280px;
  --titlebar-height: 38px;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI",
    Roboto, "Noto Sans SC", "Helvetica Neue", sans-serif;
  font-size: 13px;
  color: var(--text-primary);
  background: var(--bg-deepest);
}

html,
body,
#app {
  width: 100%;
  height: 100%;
  overflow: hidden;
  -webkit-font-smoothing: antialiased;
  user-select: none;
  -webkit-user-select: none;
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: var(--scrollbar-track);
}

::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: #3a3a3a;
}

::selection {
  background: rgba(91, 155, 213, 0.3);
}
</style>
