export interface Item {
  id: string
  file_name: string
  file_path: string
  file_size: number
  width: number
  height: number
  mime_type: string
  content_hash: string | null
  imported_at: string
}

export interface ItemQuery {
  offset?: number
  limit?: number
}

export interface ItemQueryResult {
  items: Item[]
  total: number
  offset: number
  limit: number
}

export interface ImportProgress {
  items: Item[]
  index: number
  total: number
  skipped?: number
  cancelled?: boolean
}
