import { defineStore } from "pinia"
import { ref, computed, onUnmounted } from "vue"
import { listen } from "@tauri-apps/api/event"
import { api } from "../services/api"
import type { Item, ItemQuery, ImportProgress } from "../types"
import type { UnlistenFn } from "@tauri-apps/api/event"
import { useToastStore } from "./toast"

const PAGE_SIZE = 60

export const useItemStore = defineStore("items", () => {
  const items = ref<Item[]>([])
  const total = ref(0)
  const loading = ref(false)
  const loadingMore = ref(false)
  const importing = ref(false)
  const importTotal = ref(0)
  const importDone = ref(0)
  const importFailed = ref(0)
  const importSkipped = ref(0)
  const currentId = ref<string | null>(null)
  const currentQuery = ref<ItemQuery>({})
  let unlistenImport: UnlistenFn | null = null
  let importFinalized = false
  let importItemIds = new Set<string>() // track IDs to avoid duplicates
  let thumbLoadTimer: ReturnType<typeof setTimeout> | null = null

  const current = computed(() =>
    currentId.value ? items.value.find((i) => i.id === currentId.value) ?? null : null
  )

  const hasMore = computed(() => items.value.length < total.value)

  function cleanupImport() {
    if (unlistenImport) { unlistenImport(); unlistenImport = null }
    importing.value = false
    importFinalized = false
    importItemIds.clear()
    if (thumbLoadTimer) { clearTimeout(thumbLoadTimer); thumbLoadTimer = null }
  }

  async function loadThumbsForIds(ids: string[]) {
    const batchSize = 8
    for (let i = 0; i < ids.length; i += batchSize) {
      const batch = ids.slice(i, i + batchSize)
      await Promise.all(batch.map(id => loadThumbSrc(id).catch(() => {})))
    }
  }

  async function fetchItems(query: ItemQuery = {}, append = false) {
    const q = { ...query }
    if (!append) currentQuery.value = q
    if (append) loadingMore.value = true
    else loading.value = true
    try {
      const result = await api.queryItems({ ...q, offset: append ? items.value.length : 0, limit: PAGE_SIZE })
      if (append) items.value.push(...result.items)
      else items.value = result.items
      total.value = result.total
    } finally {
      loading.value = false
      loadingMore.value = false
    }
  }

  async function loadMore() {
    if (loading.value || loadingMore.value || !hasMore.value) return
    await fetchItems(currentQuery.value, true)
  }

  async function importFiles(paths: string[]) {
    if (unlistenImport) { unlistenImport(); unlistenImport = null }

    importing.value = true
    importTotal.value = 0
    importDone.value = 0
    importFailed.value = 0
    importSkipped.value = 0
    importFinalized = false
    importItemIds.clear()

    unlistenImport = await listen<ImportProgress>("import-progress", (event) => {
      const { items: batch, index, total: evTotal, skipped = 0, cancelled } = event.payload

      if (cancelled) {
        cleanupImport()
        useToastStore().show("update", "🫡 导入已取消～")
        return
      }

      // Deduplicate finalization
      if (batch.length === 0 && importFinalized) return

      // Done signal: batch is empty, index >= total
      if (batch.length === 0 && index >= evTotal) {
        importFinalized = true
        const success = index
        importDone.value = success
        importTotal.value = success
        importFailed.value = evTotal >= success ? evTotal - success : 0
        importSkipped.value = skipped
        // Set total to actual count of items we received
        total.value = items.value.length
        cleanupImport()
        if (success > 0) {
          useToastStore().show("create", `🎉 ${success} 张新伙伴加入大家庭！`)
          // Load thumbs for items that arrived toward the end
          const ids = items.value.slice(-20).map(i => i.id).filter(id => !thumbSrcMap.has(id))
          if (ids.length > 0) loadThumbsForIds(ids)
        }
        if (skipped > 0) {
          useToastStore().show("query", `👀 跳过 ${skipped} 个无法访问的条目`)
        }
        return
      }

      // Regular progress: add new items to display immediately
      if (batch.length > 0) {
        const newItems = batch.filter(item => !importItemIds.has(item.id))
        for (const item of newItems) {
          importItemIds.add(item.id)
        }
        if (newItems.length > 0) {
          items.value.push(...newItems)
        }
        importDone.value = index
        importTotal.value = evTotal
        importSkipped.value = skipped
      }
    })

    await api.importFiles(paths)
  }

  async function cancelImport() {
    await api.cancelImport()
  }

  async function deleteItem(id: string) {
    await api.deleteItems([id])
    items.value = items.value.filter((i) => i.id !== id)
    total.value = Math.max(0, total.value - 1)
    if (currentId.value === id) currentId.value = null
  }

  onUnmounted(() => cleanupImport())

  async function clearAll() {
    await api.clearAllItems()
    items.value = []
    total.value = 0
    currentId.value = null
  }

  const thumbSrcMap = new Map<string, string>()

  function getThumbSrc(id: string): string | null {
    return thumbSrcMap.get(id) ?? null
  }

  async function loadThumbSrc(id: string): Promise<string | null> {
    if (thumbSrcMap.has(id)) return thumbSrcMap.get(id)!
    const item = items.value.find(i => i.id === id)
    if (!item) return null
    const { convertFileSrc } = await import('@tauri-apps/api/core')
    try {
      const thumbPath = await api.getThumbnailPath(id)
      if (thumbPath) {
        const url = convertFileSrc(thumbPath)
        thumbSrcMap.set(id, url)
        return url
      }
    } catch {}
    try {
      const url = convertFileSrc(item.file_path)
      thumbSrcMap.set(id, url)
      return url
    } catch { return null }
  }

  const colorCache = new Map<string, string[]>()

  async function getColors(id: string): Promise<string[]> {
    if (colorCache.has(id)) return colorCache.get(id)!
    const colors = await api.getItemColors(id)
    colorCache.set(id, colors)
    return colors
  }

  return {
    items, total, loading, loadingMore,
    importing, importTotal, importDone, importFailed, importSkipped,
    currentId, current, hasMore,
    fetchItems, loadMore, importFiles, cancelImport, deleteItem, clearAll,
    getColors, loadThumbSrc, getThumbSrc, colorCache,
  }
})
