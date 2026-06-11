<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed, nextTick, watch } from "vue"
import { useItemStore } from "../../stores/items"
import { useToastStore } from "../../stores/toast"
import { convertFileSrc } from "@tauri-apps/api/core"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import type { Item } from "../../types"

const itemStore = useItemStore()
const toast = useToastStore()
const dragOver = ref(false)
const previewItem = ref<Item | null>(null)
const zoomLevel = ref(1)
const panX = ref(0)
const panY = ref(0)
const isPanning = ref(false)
const panStartX = ref(0)
const panStartY = ref(0)
const savedScrollTop = ref(0)
const containerRef = ref<HTMLDivElement | null>(null)
const sentinelEl = ref<HTMLDivElement | null>(null)
const appWebview = getCurrentWebviewWindow()
const thumbReady = ref<Map<string, string>>(new Map())
const loadingThumbs = ref<Set<string>>(new Set())
const pkMode = ref(false)
const gifVisible = ref<Map<string, boolean>>(new Map())
const gifSrcs = ref<Map<string, string>>(new Map())
let gifObserver: IntersectionObserver | null = null

const previewIndex = computed(() => {
  if (!previewItem.value) return -1
  return itemStore.items.findIndex(i => i.id === previewItem.value!.id)
})

const pkNextIndex = computed(() => {
  const idx = previewIndex.value
  if (idx < 0 || idx >= itemStore.items.length - 1) return -1
  return idx + 1
})

const pkNextItem = computed(() => {
  const idx = pkNextIndex.value
  return idx >= 0 ? itemStore.items[idx] : null
})

let observer: IntersectionObserver | null = null

onMounted(async () => {
  await itemStore.fetchItems({})
  await nextTick()
  setupObserver()
  computeGifSrcs()
  setupGifObserver()

  const unlisten = await appWebview.onDragDropEvent(async (event) => {
    const e = event.payload
    if (e.type === "over") dragOver.value = true
    else if (e.type === "leave") dragOver.value = false
    else if (e.type === "drop") {
      dragOver.value = false
      if (e.paths.length > 0) {
        await itemStore.importFiles(e.paths)
      }
    }
  })
  onBeforeUnmount(() => {
    if (observer) { observer.disconnect(); observer = null }
    if (gifObserver) { gifObserver.disconnect(); gifObserver = null }
    unlisten()
  })
})

function setupObserver() {
  if (observer) observer.disconnect()
  if (!sentinelEl.value) return
  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0].isIntersecting) itemStore.loadMore()
    },
    { rootMargin: "600px" }
  )
  observer.observe(sentinelEl.value)
}

function setupGifObserver() {
  if (gifObserver) gifObserver.disconnect()
  gifObserver = new IntersectionObserver((entries) => {
    let changed = false
    const next = new Map(gifVisible.value)
    for (const entry of entries) {
      const id = (entry.target as HTMLElement).dataset.id
      if (id) {
        const was = next.get(id) ?? false
        const now = entry.isIntersecting
        if (was !== now) { next.set(id, now); changed = true }
      }
    }
    if (changed) gifVisible.value = next
  }, { rootMargin: "200px" })
  const container = containerRef.value
  if (container) {
    container.querySelectorAll('.grid-item[data-is-gif]')
      .forEach(el => gifObserver?.observe(el))
  }
}

function computeGifSrcs() {
  for (const item of itemStore.items) {
    if (item.mime_type === 'image/gif' && !gifSrcs.value.has(item.id)) {
      const next = new Map(gifSrcs.value)
      next.set(item.id, convertFileSrc(item.file_path))
      gifSrcs.value = next
    }
  }
}

watch(() => itemStore.items.length, async () => {
  await nextTick()
  setupObserver()
  computeGifSrcs()
  setupGifObserver()
  loadVisibleThumbs()
})

// Progressive thumbnail loading — batches of 10 at a time
async function loadVisibleThumbs() {
  const ids = itemStore.items
    .map(i => i.id)
    .filter(id => !thumbReady.value.has(id) && !loadingThumbs.value.has(id))
  const batchSize = 10
  for (let i = 0; i < ids.length; i += batchSize) {
    const batch = ids.slice(i, i + batchSize)
    await Promise.all(batch.map(async (id) => {
      if (loadingThumbs.value.has(id)) return
      loadingThumbs.value.add(id)
      const url = await itemStore.loadThumbSrc(id)
      if (url) {
        const next = new Map(thumbReady.value)
        next.set(id, url)
        thumbReady.value = next
      }
      loadingThumbs.value.delete(id)
    }))
  }
}

// ── Keyboard handling ──
function onKeydown(e: KeyboardEvent) {
  const k = e.key.toLowerCase()
  if ((k === ' ' || e.key === 'Spacebar') && !previewItem.value && itemStore.currentId) {
    const item = itemStore.items.find(i => i.id === itemStore.currentId)
    if (item) { e.preventDefault(); openPreview(item); return }
  }
  if (!previewItem.value) return
  if (e.key === "Escape") { closePreview(); e.preventDefault(); return }
  if (k === 'p') { pkMode.value = !pkMode.value; e.preventDefault(); return }
  if (pkMode.value) {
    if (e.key === "ArrowLeft" || k === 'a') { pkCurrentLoses(); e.preventDefault(); return }
    if (e.key === "ArrowRight" || k === 'd') { pkNextLoses(); e.preventDefault(); return }
  } else {
    if (e.key === "ArrowUp" || k === 'w') { keepAndPrev(); e.preventDefault(); return }
    if (e.key === "ArrowDown" || k === 's') { keepAndNext(); e.preventDefault(); return }
    if (e.key === "ArrowRight" || k === 'd') { removeAndNext(); e.preventDefault(); return }
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown))
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown))

// ── Mouse idle tip ──
const showIdleTip = ref(false)
let idleTimer: ReturnType<typeof setTimeout> | null = null
let idleCooldown = 0

function onMouseMove() {
  if (showIdleTip.value) { showIdleTip.value = false; idleCooldown = Date.now() + 15000 }
  if (idleTimer) clearTimeout(idleTimer)
  if (Date.now() < idleCooldown) return
  if (itemStore.importing || previewItem.value || itemStore.items.length === 0) return
  idleTimer = setTimeout(() => {
    if (!itemStore.importing && !previewItem.value && itemStore.items.length > 0) showIdleTip.value = true
  }, 3000)
}

onMounted(() => window.addEventListener("mousemove", onMouseMove))
onBeforeUnmount(() => {
  window.removeEventListener("mousemove", onMouseMove)
  if (idleTimer) clearTimeout(idleTimer)
})

// ── Image error — auto-remove if source gone ──
function onImageError(item: Item) {
  toast.show("delete", "😅 「" + item.file_name + "」源文件不见了，自动清理啦")
  itemStore.deleteItem(item.id)
}

function handleItemClick(item: Item) { itemStore.currentId = item.id }
function handleItemDblClick(item: Item) { openPreview(item) }

function openPreview(item: Item) {
  if (containerRef.value) savedScrollTop.value = containerRef.value.scrollTop
  previewItem.value = item
  zoomLevel.value = 1
  itemStore.currentId = item.id
}

async function closePreview() {
  const saved = savedScrollTop.value
  previewItem.value = null
  zoomLevel.value = 1
  panX.value = 0
  panY.value = 0
  isPanning.value = false
  savedScrollTop.value = 0
  await nextTick()
  if (containerRef.value && saved > 0) {
    containerRef.value.scrollTop = saved
  }
}

function keepAndPrev() {
  const idx = previewIndex.value
  if (idx > 0) {
    const prev = itemStore.items[idx - 1]
    previewItem.value = prev
    itemStore.currentId = prev.id
  }
}

function keepAndNext() {
  const idx = previewIndex.value
  if (idx < itemStore.items.length - 1) {
    const next = itemStore.items[idx + 1]
    previewItem.value = next
    itemStore.currentId = next.id
  } else { closePreview() }
}

function removeAndNext() {
  const current = previewItem.value
  if (!current) return
  const idx = previewIndex.value
  const name = current.file_name
  itemStore.deleteItem(current.id).then(() => {
    toast.show("delete", "移除了「" + name + "」")
    if (idx < itemStore.items.length) {
      previewItem.value = itemStore.items[idx]
      if (previewItem.value) itemStore.currentId = previewItem.value.id
    } else if (itemStore.items.length > 0) {
      const last = itemStore.items[itemStore.items.length - 1]
      previewItem.value = last
      itemStore.currentId = last.id
    } else { closePreview() }
  })
}

function onPreviewWheel(e: WheelEvent) {
  const delta = e.deltaY > 0 ? -0.1 : 0.1
  zoomLevel.value = Math.max(0.1, Math.min(10, zoomLevel.value + delta))
  e.preventDefault()
}

// Pan with mouse drag (when zoomed in)
function onPreviewMouseDown(e: MouseEvent) {
  if (zoomLevel.value <= 1) return
  isPanning.value = true
  panStartX.value = e.clientX - panX.value
  panStartY.value = e.clientY - panY.value
}
function onPreviewMouseMove(e: MouseEvent) {
  if (!isPanning.value) return
  panX.value = e.clientX - panStartX.value
  panY.value = e.clientY - panStartY.value
}
function onPreviewMouseUp() {
  isPanning.value = false
}

// PK mode
function pkCurrentLoses() {
  const current = previewItem.value
  const challenger = pkNextItem.value
  if (!current || !challenger) return
  const challengerId = challenger.id
  toast.show("delete", "💀 「" + current.file_name + "」败了")
  itemStore.deleteItem(current.id).then(() => {
    previewItem.value = itemStore.items.find(i => i.id === challengerId) || null
    if (previewItem.value) itemStore.currentId = previewItem.value.id
    else closePreview()
  })
}

function pkNextLoses() {
  const challenger = pkNextItem.value
  if (!challenger) return
  toast.show("delete", "💀 「" + challenger.file_name + "」败了")
  itemStore.deleteItem(challenger.id)
}
</script>

<template>
  <div class="asset-browser" :class="{ 'drag-over': dragOver }" @contextmenu.prevent>
    <Transition name="tip">
      <div v-if="showIdleTip && itemStore.items.length > 0" class="idle-tip">👆 双击图片预览大图</div>
    </Transition>

    <!-- Drop overlay -->
    <div class="drop-overlay" v-if="dragOver">
      <svg viewBox="0 0 48 48" fill="none" width="36" height="36">
        <path d="M24 4L24 32M24 32L16 24M24 32L32 24" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M8 28L8 40C8 41.1 8.9 42 10 42L38 42C39.1 42 40 41.1 40 40L40 28" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" opacity="0.5"/>
      </svg>
      <span>拖拽图片到此处</span>
    </div>

    <!-- Preview mode -->
    <div v-if="previewItem" class="preview-mode" @wheel="onPreviewWheel">
      <div class="preview-header">
        <span class="preview-name">{{ previewItem.file_name }}</span>
        <span class="preview-dim">{{ previewItem.width }} × {{ previewItem.height }}</span>
        <div class="preview-counter" v-if="itemStore.items.length > 1">
          {{ previewIndex + 1 }} / {{ itemStore.items.length }}
        </div>
        <div class="preview-zoom" v-if="zoomLevel !== 1">
          {{ Math.round(zoomLevel * 100) }}%
        </div>
        <button class="preview-close" @click="closePreview" title="关闭 (Esc)">
          <svg viewBox="0 0 16 16" fill="none" width="16" height="16">
            <path d="M12 4L4 12M4 4l8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div v-if="!pkMode" class="preview-body"
        @mousedown="onPreviewMouseDown"
        @mousemove="onPreviewMouseMove"
        @mouseup="onPreviewMouseUp"
        @mouseleave="onPreviewMouseUp">
        <img :src="convertFileSrc(previewItem.file_path)" :alt="previewItem.file_name" class="preview-image" :class="{ panning: isPanning }" :style="{ transform: 'translate(' + panX + 'px,' + panY + 'px) scale(' + zoomLevel + ')' }" />
      </div>

      <div v-else class="pk-body">
        <div class="pk-side pk-left">
          <div class="pk-label">👑 胜者</div>
          <img :src="convertFileSrc(previewItem.file_path)" :alt="previewItem.file_name" class="pk-image" />
          <div class="pk-name">{{ previewItem.file_name }}</div>
          <div class="pk-keyhint">← 或 A 淘汰胜者</div>
        </div>
        <div class="pk-vs">⚔️</div>
        <div class="pk-side pk-right" v-if="pkNextItem">
          <div class="pk-label">💪 挑战者</div>
          <img :src="convertFileSrc(pkNextItem.file_path)" :alt="pkNextItem.file_name" class="pk-image" />
          <div class="pk-name">{{ pkNextItem.file_name }}</div>
          <div class="pk-keyhint">→ 或 D 淘汰挑战者</div>
        </div>
        <div class="pk-side pk-right" v-else>
          <div class="pk-label">🏆</div>
          <div class="pk-empty">没有更多挑战者了</div>
        </div>
      </div>

      <div class="preview-actions">
        <button class="action-btn pk-toggle-btn" :class="{ active: pkMode }" @click="pkMode = !pkMode">⚔️ PK</button>
        <span class="action-sep" />
        <button v-if="!pkMode" class="action-btn prev" @click="keepAndPrev"><kbd>↑</kbd><kbd>W</kbd> 上一张</button>
        <button v-if="!pkMode" class="action-btn keep" @click="keepAndNext"><kbd>↓</kbd><kbd>S</kbd> 下一张</button>
        <button v-if="!pkMode" class="action-btn remove" @click="removeAndNext"><kbd>→</kbd><kbd>D</kbd> 移出</button>
        <button v-if="pkMode" class="action-btn pk-remove" @click="pkCurrentLoses"><kbd>←</kbd><kbd>A</kbd> 胜者败</button>
        <button v-if="pkMode" class="action-btn remove" @click="pkNextLoses"><kbd>→</kbd><kbd>D</kbd> 挑战者败</button>
      </div>
    </div>

    <!-- Grid mode -->
    <div class="grid-container" ref="containerRef">
      <template v-if="itemStore.items.length > 0">
        <div class="grid">
          <div v-for="item in itemStore.items" :key="item.id"
            class="grid-item"
            :class="{ current: item.id === itemStore.currentId }"
            :data-id="item.id"
            :data-is-gif="item.mime_type === 'image/gif' ? '' : undefined"
            @click="handleItemClick(item)"
            @dblclick="handleItemDblClick(item)">
            <div class="thumb-wrap">
              <img
                v-if="thumbReady.has(item.id)"
                :src="item.mime_type === 'image/gif' && gifSrcs.get(item.id) ? (gifVisible.get(item.id) ? gifSrcs.get(item.id) : thumbReady.get(item.id)) : thumbReady.get(item.id)"
                :alt="item.file_name"
                loading="lazy"
                draggable="false"
                @error="onImageError(item)" />
              <div v-else class="thumb-loader" />
            </div>
          </div>
        </div>
        <div ref="sentinelEl" class="sentinel" v-if="itemStore.hasMore"></div>
      </template>
      <div class="empty-state" v-else-if="!itemStore.loading && itemStore.items.length === 0">
        <div class="empty-icon">
          <svg viewBox="0 0 48 48" fill="none" width="40" height="40">
            <rect x="5" y="5" width="38" height="38" rx="4" stroke="currentColor" stroke-width="2" opacity="0.3"/>
            <path d="M24 16v16M16 24h16" stroke="currentColor" stroke-width="2" stroke-linecap="round" opacity="0.3"/>
          </svg>
        </div>
        <div class="empty-title">拖拽图片或文件夹到此处</div>
        <div class="empty-sub">支持 jpg/png/webp/gif/bmp/svg/tiff/heic/avif</div>
      </div>
      <div class="loading" v-if="itemStore.loading"><div class="spinner"></div></div>
    </div>
  </div>
</template>

<style scoped>
.asset-browser { flex:1; display:flex; flex-direction:column; overflow:hidden; position:relative; user-select:none; -webkit-user-select:none; }
.idle-tip { position:absolute; bottom:40px; left:50%; transform:translateX(-50%); z-index:200; padding:14px 28px; border-radius:12px; background:rgba(0,0,0,0.85); backdrop-filter:blur(8px); border:1px solid rgba(90,184,122,0.5); color:#80d4a0; font-size:15px; font-weight:600; pointer-events:none; white-space:nowrap; box-shadow:0 4px 24px rgba(0,0,0,0.5); }
.tip-enter-active { animation: tipIn 0.3s ease-out; }
.tip-leave-active { animation: tipOut 0.2s ease-in; }
@keyframes tipIn { from { opacity:0; transform:translateX(-50%) translateY(10px) scale(0.9); } to { opacity:1; transform:translateX(-50%) translateY(0) scale(1); } }
@keyframes tipOut { from { opacity:1; transform:translateX(-50%) translateY(0) scale(1); } to { opacity:0; transform:translateX(-50%) translateY(10px) scale(0.9); } }
.asset-browser.drag-over { background: var(--bg-surface); }

.drop-overlay { position:absolute; inset:0; z-index:100; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:8px; background:rgba(91,155,213,0.08); backdrop-filter:blur(4px); color:var(--accent); font-size:15px; pointer-events:none; }

.grid-container { flex:1; overflow-y:auto; padding:12px; }
.grid { display:grid; grid-template-columns:repeat(auto-fill,minmax(140px,1fr)); gap:8px; }
.grid-item { aspect-ratio:1; content-visibility:auto; contain-intrinsic-size:140px; cursor:pointer; position:relative; border-radius:4px; overflow:hidden; transition:box-shadow 0.15s; }
.grid-item:hover { box-shadow:0 0 0 1px rgba(255,255,255,0.15); }
.grid-item.current { 
  box-shadow:0 0 0 3px #ff0000, 0 0 12px rgba(224,96,96,0.4); 
  z-index:1;
}
.thumb-wrap { width:100%; height:100%; overflow:hidden; position:relative; background:var(--bg-deepest); border-radius:4px; }
.thumb-wrap img { width:100%; height:100%; object-fit:cover; display:block; }
.thumb-loader { width:100%; height:100%; background:var(--bg-surface); }

.sentinel { height:1px; }
.empty-state { display:flex; flex-direction:column; align-items:center; justify-content:center; height:100%; gap:8px; color:var(--text-muted); pointer-events:none; }
.empty-icon { opacity:0.5; margin-bottom:8px; }
.empty-title { font-size:15px; color:var(--text-secondary); }
.empty-sub { font-size:12px; }
.loading { display:flex; align-items:center; justify-content:center; padding:40px; }
.spinner { width:24px; height:24px; border:2px solid var(--border); border-top-color:var(--accent); border-radius:50%; animation:spin 0.7s linear infinite; }
@keyframes spin { to { transform:rotate(360deg); } }

.preview-mode { position:absolute; inset:0; z-index:50; display:flex; flex-direction:column; overflow:hidden; background:var(--bg-main); user-select:none; }
.preview-header { display:flex; align-items:center; gap:12px; padding:10px 14px; background:var(--bg-surface); border-bottom:1px solid var(--border); flex-shrink:0; }
.preview-name { font-size:14px; font-weight:600; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.preview-dim { font-size:11px; color:var(--text-muted); flex-shrink:0; }
.preview-counter { font-size:11px; color:var(--text-muted); background:var(--bg-hover); padding:2px 8px; border-radius:4px; flex-shrink:0; }
.preview-zoom { font-size:11px; color:var(--accent3); background:rgba(210,153,34,0.12); padding:2px 8px; border-radius:4px; flex-shrink:0; }
.preview-close { display:flex; align-items:center; justify-content:center; width:32px; height:32px; border-radius:6px; background:transparent; border:none; color:#aaa; cursor:pointer; transition:all .12s; flex-shrink:0; margin-left:auto; }
.preview-close:hover { background:rgba(255,255,255,0.1); color:#fff; }
.preview-body { flex:1; display:flex; align-items:center; justify-content:center; overflow:hidden; background:var(--bg-deepest); }
.preview-image { max-width:100%; max-height:100%; object-fit:contain; border-radius:4px; transition:transform 0.08s ease-out; image-rendering:pixelated; image-rendering:-moz-crisp-edges; image-rendering:crisp-edges; cursor:grab; }
.preview-image.panning { cursor:grabbing; transition:none; }
.preview-actions { display:flex; justify-content:center; gap:8px; padding:10px; background:var(--bg-surface); border-top:1px solid var(--border); flex-shrink:0; }

.action-sep { width:1px; height:20px; background:var(--border); flex-shrink:0; }
.pk-toggle-btn { gap:4px !important; }
.pk-toggle-btn.active { background:rgba(180,100,60,0.2) !important; border-color:rgba(180,100,60,0.5) !important; color:#e8a87c !important; }

.pk-body { flex:1; display:flex; align-items:center; justify-content:center; gap:24px; overflow:hidden; background:var(--bg-deepest); padding:16px; }
.pk-side { display:flex; flex-direction:column; align-items:center; gap:8px; flex:1; max-width:45%; }
.pk-label { font-size:11px; font-weight:700; color:var(--text-muted); letter-spacing:1px; }
.pk-image { max-width:100%; max-height:calc(100vh - 200px); object-fit:contain; border-radius:8px; }
.pk-name { font-size:11px; color:var(--text-secondary); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; max-width:100%; text-align:center; }
.pk-keyhint { font-size:10px; color:var(--text-muted); opacity:0.6; }
.pk-vs { font-size:28px; flex-shrink:0; opacity:0.4; }
.pk-empty { font-size:13px; color:var(--text-muted); padding:40px 0; }
.pk-remove { color:#e8a87c; border-color:rgba(180,100,60,0.3); }
.pk-remove:hover { background:rgba(180,100,60,0.15); border-color:rgba(180,100,60,0.5); color:#e8a87c; }
.action-btn { display:flex; align-items:center; gap:6px; padding:6px 14px; border-radius:6px; font-size:12px; cursor:pointer; border:1px solid var(--border); background:var(--bg-hover); color:var(--text-secondary); transition:all.12s; }
.action-btn:hover { background:var(--bg-active); color:var(--text-primary); }
.action-btn.remove:hover { background:rgba(212,90,90,0.15); border-color:rgba(212,90,90,0.3); color:#f0a0a0; }
.action-btn kbd { display:inline-flex; align-items:center; justify-content:center; min-width:20px; height:20px; padding:0 5px; border-radius:3px; background:var(--bg-deepest); border:1px solid var(--border); font-family:inherit; font-size:11px; color:var(--text-secondary); }
</style>
