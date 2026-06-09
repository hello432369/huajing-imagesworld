<script setup lang="ts">
import { ref, computed, watch } from "vue"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { useItemStore } from "../../stores/items"
import { useToastStore } from "../../stores/toast"

interface ExtractedColor {
  hex: string
  rgb: string
  hsl: string
}

interface ColorDetail {
  label: string
  value: string
}

const itemStore = useItemStore()
const toast = useToastStore()
const appWindow = getCurrentWindow()

const colors = ref<ExtractedColor[]>([])
const extracting = ref(false)
const selectedColor = ref<string | null>(null)
const showFormats = ref(false)

const item = computed(() => itemStore.current)

const itemIndex = computed(() => {
  if (!item.value) return 0
  const idx = itemStore.items.findIndex(i => i.id === item.value!.id)
  return idx >= 0 ? idx + 1 : 0
})

const colorDetails = computed<ColorDetail[]>(() => {
  if (!selectedColor.value) return []
  const hex = selectedColor.value
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  return [
    { label: "HEX", value: hex },
    { label: "RGB", value: `rgb(${r}, ${g}, ${b})` },
    { label: "RGBA", value: `rgba(${r}, ${g}, ${b}, 1)` },
    { label: "HSL", value: rgbToHsl(r, g, b) },
    { label: "HSV", value: rgbToHsv(r, g, b) },
    { label: "HWB", value: rgbToHwb(r, g, b) },
    { label: "CMYK", value: rgbToCmyk(r, g, b) },
  ]
})

watch(() => item.value?.id, async (id) => {
  if (!id || !item.value) { colors.value = []; return }
  selectedColor.value = null
  extracting.value = true
  try {
    const hexColors = await itemStore.getColors(id)
    colors.value = hexColors.map(h => ({
      hex: h,
      rgb: '',
      hsl: '',
    }))
    if (colors.value.length > 0) selectedColor.value = colors.value[0].hex
  } catch (e) {
    console.warn('getColors failed:', e)
  } finally {
    extracting.value = false
  }
})

async function copyMeta(label: string, value: string) {
  try {
    await navigator.clipboard.writeText(value)
    const preview = value.length > 30 ? value.substring(0, 30) + "..." : value
    toast.show("query", "已复制 " + label + "：「" + preview + "」")
  } catch { /* ignore */ }
}

function formatDate(ts: string): string {
  try {
    const d = new Date(ts)
    return d.toLocaleDateString("zh-CN", {
      year: "numeric", month: "2-digit", day: "2-digit",
      hour: "2-digit", minute: "2-digit",
    })
  } catch { return ts }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B"
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB"
  return (bytes / (1024 * 1024)).toFixed(1) + " MB"
}

// ── Color space conversions ──
function rgbToHex(r: number, g: number, b: number): string {
  return "#" + [r, g, b].map(c => c.toString(16).padStart(2, "0")).join("")
}

function rgbToHsl(r: number, g: number, b: number): string {
  const rr = r / 255, gg = g / 255, bb = b / 255
  const max = Math.max(rr, gg, bb), min = Math.min(rr, gg, bb)
  let h = 0, s = 0, l = (max + min) / 2
  if (max !== min) {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    switch (max) {
      case rr: h = ((gg - bb) / d + (gg < bb ? 6 : 0)) / 6; break
      case gg: h = ((bb - rr) / d + 2) / 6; break
      case bb: h = ((rr - gg) / d + 4) / 6; break
    }
  }
  return `hsl(${(h * 360).toFixed(0)}, ${(s * 100).toFixed(0)}%, ${(l * 100).toFixed(0)}%)`
}

function rgbToHsv(r: number, g: number, b: number): string {
  const rr = r / 255, gg = g / 255, bb = b / 255
  const max = Math.max(rr, gg, bb), min = Math.min(rr, gg, bb)
  const v = max, d = max - min
  const s = max === 0 ? 0 : d / max
  let h = 0
  if (max !== min) {
    switch (max) {
      case rr: h = ((gg - bb) / d + (gg < bb ? 6 : 0)) / 6; break
      case gg: h = ((bb - rr) / d + 2) / 6; break
      case bb: h = ((rr - gg) / d + 4) / 6; break
    }
  }
  return `hsv(${(h * 360).toFixed(0)}, ${(s * 100).toFixed(0)}%, ${(v * 100).toFixed(0)}%)`
}

function rgbToHwb(r: number, g: number, b: number): string {
  const rr = r / 255, gg = g / 255, bb = b / 255
  const max = Math.max(rr, gg, bb), min = Math.min(rr, gg, bb)
  const d = max - min
  let h = 0
  if (max !== min) {
    switch (max) {
      case rr: h = ((gg - bb) / d + (gg < bb ? 6 : 0)) / 6; break
      case gg: h = ((bb - rr) / d + 2) / 6; break
      case bb: h = ((rr - gg) / d + 4) / 6; break
    }
  }
  return `hwb(${(h * 360).toFixed(0)}, ${(min * 100).toFixed(0)}%, ${((1 - max) * 100).toFixed(0)}%)`
}

function rgbToCmyk(r: number, g: number, b: number): string {
  const rr = r / 255, gg = g / 255, bb = b / 255
  const k = 1 - Math.max(rr, gg, bb)
  if (k === 1) return "cmyk(0%, 0%, 0%, 100%)"
  const c = ((1 - rr - k) / (1 - k)) * 100
  const m = ((1 - gg - k) / (1 - k)) * 100
  const y = ((1 - bb - k) / (1 - k)) * 100
  return `cmyk(${c.toFixed(0)}%, ${m.toFixed(0)}%, ${y.toFixed(0)}%, ${(k * 100).toFixed(0)}%)`
}

</script>

<template>
  <aside class="inspector">
    <!-- Traffic lights + title at top -->
    <div class="inspector-top" data-tauri-drag-region>
      <div class="traffic-lights" style="-webkit-app-region: no-drag; app-region: no-drag;">
        <button class="traffic-dot close" @click="appWindow.close()" title="关闭" style="-webkit-app-region: no-drag; app-region: no-drag;" />
        <button class="traffic-dot minimize" @click="appWindow.minimize()" title="最小化" style="-webkit-app-region: no-drag; app-region: no-drag;" />
        <button class="traffic-dot zoom" @click="appWindow.toggleMaximize()" title="缩放" style="-webkit-app-region: no-drag; app-region: no-drag;" />
      </div>
      <span class="brand">画境</span>
    </div>

    <div class="inspector-body" v-if="item">
      <!-- Colors -->
      <div class="section">
        <div class="section-label">色彩提取</div>
        <div class="loading-text" v-if="extracting">🎨 提取中...</div>
        <template v-if="colors.length > 0">
          <div class="palette-row">
            <div
              v-for="(c, i) in colors"
              :key="i"
              class="swatch"
              :class="{ active: selectedColor === c.hex }"
              :style="{ background: c.hex }"
              @click="selectedColor = c.hex"
              :title="c.hex"
            />
          </div>
          <div class="color-detail" v-if="colorDetails.length > 0">
            <div class="detail-row" v-for="d in colorDetails" :key="d.label">
              <span class="detail-label">{{ d.label }}</span>
              <span class="detail-value">{{ d.value }}</span>
              <button class="copy-btn" @click="copyMeta(d.label, d.value)">{{ '复制' }}</button>
            </div>
          </div>
        </template>
        <div class="loading-text" v-else-if="!extracting">点击图片自动提取</div>
      </div>

      <!-- Metadata -->
      <div class="section">
        <div class="section-label">文件信息</div>
        <div class="meta-grid">
          <div class="meta-row" @click="copyMeta('序号', itemIndex + ' / ' + itemStore.total)" title="点击复制">
            <span class="meta-key">序号</span>
            <span class="meta-val">{{ itemIndex }} / {{ itemStore.total }}</span>
          </div>
          <div class="meta-row" @click="copyMeta('文件名', item.file_name)" title="点击复制">
            <span class="meta-key">文件名</span>
            <span class="meta-val">{{ item.file_name }}</span>
          </div>
          <div class="meta-row" @click="copyMeta('尺寸', item.width + ' × ' + item.height)" title="点击复制">
            <span class="meta-key">尺寸</span>
            <span class="meta-val">{{ item.width }} × {{ item.height }}</span>
          </div>
          <div class="meta-row" @click="copyMeta('大小', formatSize(item.file_size))" title="点击复制">
            <span class="meta-key">大小</span>
            <span class="meta-val">{{ formatSize(item.file_size) }}</span>
          </div>
          <div class="meta-row" @click="copyMeta('类型', item.mime_type)" title="点击复制">
            <span class="meta-key">类型</span>
            <span class="meta-val">{{ item.mime_type }}</span>
          </div>
          <div class="meta-row" @click="copyMeta('路径', item.file_path)" title="点击复制">
            <span class="meta-key">路径</span>
            <span class="meta-val path">{{ item.file_path }}</span>
          </div>
          <div class="meta-row" @click="copyMeta('导入时间', formatDate(item.imported_at))" title="点击复制">
            <span class="meta-key">导入时间</span>
            <span class="meta-val">{{ formatDate(item.imported_at) }}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="inspector-empty" v-else />
    
    <!-- Help / Supported formats -->
    <div class="inspector-footer">
      <button class="help-btn" @click="showFormats = !showFormats" title="支持的格式">
        <svg viewBox="0 0 20 20" fill="none" width="16" height="16">
          <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.5"/>
          <path d="M8 8c0-1.1.9-2 2-2s2 .9 2 2c0 1.5-2 2-2 3.5V13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <circle cx="10" cy="15" r=".8" fill="currentColor"/>
        </svg>
      </button>
    </div>

    <!-- Format info panel -->
    <Transition name="popup">
      <div v-if="showFormats" class="format-popup">
        <div class="format-header">
          <span>支持的图片格式</span>
          <button class="format-close" @click="showFormats = false">
            <svg viewBox="0 0 16 16" fill="none" width="14" height="14">
              <path d="M12 4L4 12M4 4l8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
        <div class="format-body">
          <div class="format-group">
            <div class="format-group-title">📷 静态位图</div>
            <div class="format-tags">
              <span class="format-tag" title=".jpg .jpeg .jfif .jpe">JPEG</span>
              <span class="format-tag" title=".png">PNG</span>
              <span class="format-tag" title=".webp">WebP</span>
              <span class="format-tag" title=".bmp">BMP</span>
              <span class="format-tag" title=".tiff .tif">TIFF</span>
              <span class="format-tag" title=".avif">AVIF</span>
              <span class="format-tag" title=".heic .heif">HEIC</span>
              <span class="format-tag" title=".ico .cur">ICO</span>
            </div>
          </div>
          <div class="format-group">
            <div class="format-group-title">🎨 矢量图</div>
            <div class="format-tags">
              <span class="format-tag" title=".svg">SVG</span>
            </div>
          </div>
          <div class="format-group">
            <div class="format-group-title">🔄 动图</div>
            <div class="format-tags">
              <span class="format-tag" title=".gif">GIF</span>
            </div>
          </div>
          <div class="format-note">拖拽图片或文件夹到窗口即可导入 · 自动识别文件头</div>
        </div>
      </div>
    </Transition>
  </aside>
</template>

<style scoped>
.inspector {
  width: 280px;
  min-width: 280px;
  background: var(--bg-deepest);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
  flex-shrink: 0;
}

.inspector-top {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  flex-shrink: 0;
  -webkit-app-region: drag;
  app-region: drag;
}

.traffic-lights {
  display: flex;
  gap: 7px;
  align-items: center;
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
.traffic-dot:hover { opacity: 0.7; }
.close { background: #ff5f57; }
.minimize { background: #febc2e; }
.zoom { background: #28c840; }

.brand {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.5px;
  pointer-events: none;
}

.inspector-body { flex: 1; overflow-y: auto; }

.section { padding: 12px 14px 14px; }
.section + .section { border-top: 1px solid var(--border); }

.section-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 8px;
}

.loading-text { font-size: 12px; color: var(--text-muted); padding: 8px 0; }

.palette-row {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
  flex-wrap: wrap;
}

.swatch {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  cursor: pointer;
  border: 2px solid transparent;
  transition: all 0.12s;
  flex-shrink: 0;
}
.swatch:hover { transform: scale(1.15); z-index: 1; }
.swatch.active { border-color: #fff; box-shadow: 0 0 0 2px rgba(0,0,0,0.3); transform: scale(1.1); }

.color-detail {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
  position: relative;
}

.detail-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  font-family: "SF Mono", "JetBrains Mono", "Fira Code", monospace;
  font-size: 11px;
  border-bottom: 1px solid var(--border);
}
.detail-row:last-child { border-bottom: none; }

.detail-label {
  color: var(--text-muted);
  min-width: 36px;
  font-size: 10px;
  font-weight: 600;
  flex-shrink: 0;
}

.detail-value {
  flex: 1;
  color: var(--text-primary);
  overflow: hidden;
  position: relative;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.copy-btn {
  padding: 2px 6px;
  border-radius: 4px;
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 10px;
  cursor: pointer;
  transition: all 0.1s;
  flex-shrink: 0;
}
.copy-btn:hover { background: var(--bg-hover); color: var(--accent); border-color: var(--accent); }

.meta-grid { display: flex; flex-direction: column; }

.meta-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 0;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background 0.1s;
  border-radius: 3px;
  padding-left: 4px;
  padding-right: 4px;
}
.meta-row:last-child { border-bottom: none; }
.meta-row:hover { background: var(--bg-hover); }

.meta-key {
  font-size: 11px;
  color: var(--text-muted);
  min-width: 52px;
  flex-shrink: 0;
}

.meta-val {
  font-size: 12px;
  color: var(--text-primary);
  word-break: break-all;
  overflow: hidden;
  position: relative;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.meta-val.path { font-size: 11px; color: var(--text-secondary); }

.inspector-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 12px;
}

.inspector-footer {
  flex-shrink: 0;
  padding: 6px 14px;
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: flex-end;
}

.help-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.12s;
}
.help-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

.format-popup {
  position: fixed;
  bottom: 60px;
  left: 14px;
  width: 252px;
  max-height: 420px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.6);
  z-index: 9999;
  display: flex;
  flex-direction: column;
}

.format-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
}

.format-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}
.format-close:hover { background: var(--bg-hover); color: var(--text-primary); }

.format-body { padding: 10px 12px 12px; overflow-y: auto; }

.format-group { margin-bottom: 10px; }
.format-group:last-child { margin-bottom: 0; }

.format-group-title {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  margin-bottom: 5px;
}

.format-tags { display: flex; flex-wrap: wrap; gap: 4px; }

.format-tag {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
  background: var(--bg-hover);
  color: var(--text-secondary);
  border: 1px solid var(--border);
  letter-spacing: 0.3px;
}

.format-note {
  margin-top: 8px;
  font-size: 10px;
  color: var(--text-muted);
  text-align: center;
}

.popup-enter-active { animation: popupIn 0.15s ease-out; }
.popup-leave-active { animation: popupOut 0.1s ease-in; }
@keyframes popupIn { from { opacity: 0; transform: translateY(6px) scale(0.95); } to { opacity: 1; transform: translateY(0) scale(1); } }
@keyframes popupOut { from { opacity: 1; transform: translateY(0) scale(1); } to { opacity: 0; transform: translateY(6px) scale(0.95); } }
</style>
