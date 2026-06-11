<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue"
import { open as openDialog } from "@tauri-apps/plugin-dialog"
import { useItemStore } from "../../stores/items"
import { useToastStore } from "../../stores/toast"
import { api } from "../../services/api"

const itemStore = useItemStore()
const toast = useToastStore()

const visible = ref(false)
const x = ref(0)
const y = ref(0)
const showConfirm = ref(false)
const exporting = ref(false)

function open(event: MouseEvent) {
  x.value = event.clientX
  y.value = event.clientY
  visible.value = true
  showConfirm.value = false
}

function close() {
  visible.value = false
  showConfirm.value = false
}

function handleClearAll() {
  visible.value = false
  showConfirm.value = true
}

async function handleConfirm() {
  try {
    await itemStore.clearAll()
    toast.show("delete", "好嘞～全部清空，干干净净！")
  } catch (e) {
    console.error("clearAll error:", e)
    toast.show("update", "😵 清空失败: " + e)
  }
  close()
}

function handleCancel() {
  close()
}

async function handleExport() {
  visible.value = false
  exporting.value = true

  try {
    const dir = await openDialog({
      directory: true,
      multiple: false,
      title: "选择导出文件夹",
    })
    if (!dir) {
      exporting.value = false
      return // user cancelled
    }

    const count = await api.exportItems(dir)
    toast.show("create", `📦 已移动 ${count} 张图片到目标文件夹！`)
    exporting.value = false
  } catch (e: any) {
    toast.show("update", `😵 导出出错了: ${e}`)
    exporting.value = false
  }
}

function onGlobalClick() {
  if (visible.value && !showConfirm.value) close()
}

onMounted(() => window.addEventListener("click", onGlobalClick))
onBeforeUnmount(() => window.removeEventListener("click", onGlobalClick))

defineExpose({ open, close })
</script>

<template>
  <Teleport to="body">
    <Transition name="menu">
      <div v-if="visible && !showConfirm" class="ctx-menu" :style="{ left: x + 'px', top: y + 'px' }">
        <div class="ctx-item" @click="handleExport">
          <span class="ctx-icon">📁</span>
          <span>导出全部到文件夹</span>
        </div>
        <div class="ctx-sep"></div>
        <div class="ctx-item danger" @click="handleClearAll">
          <span class="ctx-icon">🗑️</span>
          <span>清空全部图片</span>
        </div>
      </div>
    </Transition>

    <Transition name="confirm">
      <div v-if="showConfirm" class="confirm-overlay" @click="handleCancel">
        <div class="confirm-dialog">
          <div class="big-emoji" @click="handleConfirm">🤔</div>
          <div class="confirm-actions">
            <button class="confirm-btn cancel" @click.stop="handleCancel">取消</button>
            <button class="confirm-btn ok" @click.stop="handleConfirm">确定清空</button>
          </div>
        </div>
      </div>
    </Transition>

    <Transition name="confirm">
      <div v-if="exporting" class="confirm-overlay">
        <div class="export-dialog">
          <div class="export-spinner"></div>
          <div class="export-text">正在移动文件…</div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.ctx-menu {
  position: fixed;
  z-index: 20000;
  min-width: 160px;
  background: var(--bg-surface, #1c1c1c);
  border: 1px solid var(--border, #252525);
  border-radius: 8px;
  padding: 4px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.6);
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
  transition: background 0.1s;
}

.ctx-item:hover { background: var(--bg-hover, #232323); }
.ctx-item.danger { color: #f0a0a0; }
.ctx-item.danger:hover { background: rgba(212,90,90,0.15); }
.ctx-icon { font-size: 15px; line-height: 1; }
.ctx-sep { height: 1px; background: var(--border, #252525); margin: 4px 8px; }

.menu-enter-active { animation: menuIn 0.12s ease-out; }
.menu-leave-active { animation: menuOut 0.1s ease-in; }

@keyframes menuIn {
  from { opacity: 0; transform: scale(0.95) translateY(-4px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
@keyframes menuOut {
  from { opacity: 1; transform: scale(1) translateY(0); }
  to { opacity: 0; transform: scale(0.95) translateY(-4px); }
}

.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 30000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0,0,0,0.5);
  backdrop-filter: blur(6px);
}

.confirm-dialog {
  background: rgba(28,28,28,0.95);
  border: 1px solid var(--border, #252525);
  border-radius: 24px;
  padding: 32px 36px 24px;
  text-align: center;
  box-shadow: 0 16px 64px rgba(0,0,0,0.5);
}

.big-emoji {
  font-size: 64px;
  line-height: 1;
  margin-bottom: 20px;
  cursor: pointer;
  filter: drop-shadow(0 0 20px rgba(255,255,255,0.08));
  transition: transform 0.1s;
}
.big-emoji:hover { transform: scale(1.1); }
.big-emoji:active { transform: scale(0.9); }

.confirm-actions { display: flex; gap: 12px; justify-content: center; }

.confirm-btn {
  padding: 8px 22px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  border: none;
  transition: all 0.12s;
}

.confirm-btn.cancel {
  background: var(--bg-hover, #232323);
  color: var(--text-secondary);
  border: 1px solid var(--border, #252525);
}

.confirm-btn.cancel:hover {
  background: var(--bg-active, #2a2a2a);
  color: var(--text-primary);
}

.confirm-btn.ok {
  background: rgba(212,90,90,0.2);
  color: #f0a0a0;
  border: 1px solid rgba(212,90,90,0.4);
}

.confirm-btn.ok:hover {
  background: rgba(212,90,90,0.3);
}

.confirm-enter-active { animation: confirmIn 0.2s ease-out; }
.confirm-leave-active { animation: confirmOut 0.15s ease-in; }

@keyframes confirmIn {
  from { opacity: 0; transform: scale(0.7); }
  to { opacity: 1; transform: scale(1); }
}
@keyframes confirmOut {
  from { opacity: 1; transform: scale(1); }
  to { opacity: 0; transform: scale(0.7); }
}

.export-dialog {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  background: rgba(28,28,28,0.95);
  border: 1px solid var(--border, #252525);
  border-radius: 24px;
  padding: 40px 48px;
  box-shadow: 0 16px 64px rgba(0,0,0,0.5);
}

.export-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.export-text {
  font-size: 14px;
  color: var(--text-secondary);
}
</style>