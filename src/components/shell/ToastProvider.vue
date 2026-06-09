<script setup lang="ts">
import { useToastStore } from "../../stores/toast"

const toastStore = useToastStore()

const emojiMap: Record<string, string> = {
  create: "🎉",
  delete: "😅",
  update: "✨",
  query: "👀",
}
</script>

<template>
  <div class="toast-area">
    <TransitionGroup name="toast" class="toast-stack">
      <div
        v-for="toast in toastStore.toasts"
        :key="toast.id"
        class="toast-item"
        :class="'toast-' + toast.type"
      >
        <span class="toast-emoji">{{ emojiMap[toast.type] || "💬" }}</span>
        <span class="toast-msg">{{ toast.message }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-area {
  position: fixed;
  left: var(--inspector-width, 280px);
  right: 0;
  bottom: 60px;
  z-index: 10000;
  display: flex;
  justify-content: center;
  pointer-events: none;
}

.toast-stack {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  max-width: 420px;
}

.toast-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  border-radius: 12px;
  font-size: 13.5px;
  font-weight: 600;
  box-shadow: 0 4px 24px rgba(0,0,0,0.6);
  pointer-events: auto;
  white-space: nowrap;
}

.toast-create { background: rgba(30,60,45,0.94); border: 1px solid rgba(90,184,122,0.5); color: #80d4a0; }
.toast-delete { background: rgba(70,25,25,0.94); border: 1px solid rgba(212,90,90,0.5); color: #f0b0b0; }
.toast-update { background: rgba(25,45,75,0.94); border: 1px solid rgba(91,155,213,0.5); color: #80bce0; }
.toast-query { background: rgba(65,45,15,0.94); border: 1px solid rgba(210,153,34,0.5); color: #e0c070; }

.toast-emoji { font-size: 18px; line-height: 1; flex-shrink: 0; }
.toast-msg { color: var(--text-primary); }

.toast-enter-active { animation: toastBounceIn 0.35s cubic-bezier(0.34,1.56,0.64,1); }
.toast-leave-active { animation: toastFadeOut 0.2s ease-in; }

@keyframes toastBounceIn {
  from { opacity: 0; transform: translateY(20px) scale(0.85); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
@keyframes toastFadeOut {
  from { opacity: 1; transform: translateY(0) scale(1); }
  to { opacity: 0; transform: translateY(-6px) scale(0.95); }
}
</style>
