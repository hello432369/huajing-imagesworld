<script setup lang="ts">
import { ref } from "vue"
import { useItemStore } from "../../stores/items"
import Inspector from "./Inspector.vue"
import AssetBrowser from "./AssetBrowser.vue"
import ToastProvider from "./ToastProvider.vue"
import ContextMenu from "./ContextMenu.vue"

const itemStore = useItemStore()
const ctxMenu = ref<InstanceType<typeof ContextMenu> | null>(null)

function onContextMenu(e: MouseEvent) {
  e.preventDefault()
  ctxMenu.value?.open(e)
}
</script>

<template>
  <div class="app-shell" @contextmenu="onContextMenu">
    <div class="app-body">
      <Inspector />
      <main class="content-area">
        <AssetBrowser />
      </main>
    </div>
    <ToastProvider />
    <ContextMenu ref="ctxMenu" />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: var(--bg-main);
}

.app-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.content-area {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
</style>
