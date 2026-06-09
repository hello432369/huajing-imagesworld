/// <reference types="vite/client" />

interface Item {
  id: string
  fileName: string
  filePath: string
  width: number
  height: number
  mimeType: string
  tags: string[]
  rating: number
}

interface Folder {
  id: string
  name: string
  parentId: string | null
}

interface PluginAPI {
  items: {
    query(folderId?: string): Promise<Item[]>
    get(id: string): Promise<Item | null>
    import(paths: string[], folderId: string): Promise<Item[]>
  }
  folders: {
    list(): Promise<Folder[]>
    create(name: string, parentId?: string): Promise<Folder>
    remove(id: string): Promise<void>
    rename(id: string, name: string): Promise<Folder>
  }
  tags: {
    list(): Promise<{ name: string; count: number }[]>
    add(itemId: string, tags: string[]): Promise<void>
    remove(itemId: string, tags: string[]): Promise<void>
  }
  engine: {
    getThumbnail(itemId: string, size: number): Promise<string>
  }
  events: {
    on(event: string, handler: (data: unknown) => void): void
    off(event: string, handler: (data: unknown) => void): void
    emit(event: string, data: unknown): void
  }
  panels: {
    setTitle(title: string): void
  }
  commands: {
    execute(commandId: string, args?: unknown): Promise<unknown>
  }
}

type MessageHandler = (data: unknown) => void

const _handlers = new Map<string, Set<MessageHandler>>()

function _post(action: string, payload?: unknown) {
  window.parent.postMessage({ action, payload }, "*")
}

function _onMessage(event: MessageEvent) {
  const { action, payload } = event.data ?? {}
  if (!action) return
  const handlers = _handlers.get(action)
  if (handlers) {
    handlers.forEach((h) => h(payload))
  }
}

window.addEventListener("message", _onMessage)

export const eaglet: PluginAPI = {
  items: {
    async query(folderId) {
      _post("items:query", { folderId })
      return []
    },
    async get(id) {
      _post("items:get", { id })
      return null
    },
    async import(paths, folderId) {
      _post("items:import", { paths, folderId })
      return []
    },
  },

  folders: {
    async list() {
      _post("folders:list")
      return []
    },
    async create(name, parentId) {
      _post("folders:create", { name, parentId })
      return { id: "", name, parentId: parentId ?? null }
    },
    async remove(id) {
      _post("folders:remove", { id })
    },
    async rename(id, name) {
      _post("folders:rename", { id, name })
      return { id, name, parentId: null }
    },
  },

  tags: {
    async list() {
      _post("tags:list")
      return []
    },
    async add(itemId, tags) {
      _post("tags:add", { itemId, tags })
    },
    async remove(itemId, tags) {
      _post("tags:remove", { itemId, tags })
    },
  },

  engine: {
    async getThumbnail(itemId, size) {
      _post("engine:thumbnail", { itemId, size })
      return ""
    },
  },

  events: {
    on(event, handler) {
      if (!_handlers.has(event)) _handlers.set(event, new Set())
      _handlers.get(event)!.add(handler)
    },
    off(event, handler) {
      _handlers.get(event)?.delete(handler)
    },
    emit(event, data) {
      _post("event:" + event, data)
    },
  },

  panels: {
    setTitle(title) {
      _post("panels:setTitle", { title })
    },
  },

  commands: {
    async execute(commandId, args) {
      _post("commands:execute", { commandId, args })
      return undefined
    },
  },
}

export function onPluginEvent(type: string, handler: (data: unknown) => void) {
  eaglet.events.on(type, handler)
}

export function notifyError(message: string) {
  console.error("[画境插件]", message)
}
