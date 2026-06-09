import { defineStore } from "pinia"
import { ref } from "vue"

export type ToastType = "create" | "delete" | "update" | "query"

export interface Toast {
  id: number
  type: ToastType
  message: string
}

export const useToastStore = defineStore("toast", () => {
  const toasts = ref<Toast[]>([])
  let nextId = 1

  function show(type: ToastType, message: string, duration = 2000) {
    // 同类型替换，不堆叠
    const existing = toasts.value.find((t) => t.type === type)
    if (existing) {
      toasts.value = toasts.value.filter((t) => t.type !== type)
    }
    const id = nextId++
    toasts.value.push({ id, type, message })
    setTimeout(() => {
      toasts.value = toasts.value.filter((t) => t.id !== id)
    }, duration)
  }

  return { toasts, show }
})
