import { invoke } from "@tauri-apps/api/core"
import type { Item, ItemQuery, ItemQueryResult } from "../types"

export const api = {
  queryItems: (query: ItemQuery) => invoke<ItemQueryResult>("query_items", { query }),
  importFiles: (paths: string[]) => invoke<void>("import_files", { paths }),
  cancelImport: () => invoke<void>("cancel_import"),
  deleteItems: (ids: string[]) => invoke<void>("delete_items", { ids }),
  clearAllItems: () => invoke<void>("clear_all_items"),
  getThumbnailPath: (id: string) => invoke<string | null>("get_thumbnail_path", { id }),
  getItemColors: (id: string) => invoke<string[]>("get_item_colors", { id }),
  exportItems: (destPath: string) => invoke<number>("export_items", { destPath }),
}
