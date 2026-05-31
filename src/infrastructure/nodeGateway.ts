import { invoke } from "@tauri-apps/api/core";

import type { ClueNode } from "../domain/clueNode";
import { normalizeNode } from "../domain/clueNode";
import { canonicalShapeId } from "../domain/shapeMapper";

export const BROWSER_STORAGE_KEY = "cyberweaver.browser.nodes";

type RuntimeMode = "tauri" | "browser";

interface NodeGateway {
  listNodes(): Promise<ClueNode[]>;
  upsertNodes(nodes: ClueNode[]): Promise<void>;
  deleteNodes(ids: string[]): Promise<void>;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const memoryStorage = new Map<string, string>();

function getStorageItem(key: string): string | null {
  if (typeof window === "undefined") {
    return memoryStorage.get(key) ?? null;
  }

  return window.localStorage.getItem(key);
}

function setStorageItem(key: string, value: string): void {
  if (typeof window === "undefined") {
    memoryStorage.set(key, value);
    return;
  }

  window.localStorage.setItem(key, value);
}

function removeStorageItem(key: string): void {
  if (typeof window === "undefined") {
    memoryStorage.delete(key);
    return;
  }

  window.localStorage.removeItem(key);
}

function normalizeNodes(raw: unknown): ClueNode[] {
  if (!Array.isArray(raw)) {
    return [];
  }

  return raw
    .map((entry) => normalizeNode(entry))
    .filter((entry): entry is ClueNode => entry !== null)
    .map((entry) => ({
      ...entry,
      id: String(canonicalShapeId(entry.id)),
    }));
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && typeof window.__TAURI_INTERNALS__ !== "undefined";
}

class BrowserNodeGateway implements NodeGateway {
  private readAll(): ClueNode[] {
    const raw = getStorageItem(BROWSER_STORAGE_KEY);

    if (!raw) {
      return [];
    }

    try {
      return normalizeNodes(JSON.parse(raw));
    } catch {
      removeStorageItem(BROWSER_STORAGE_KEY);
      return [];
    }
  }

  private writeAll(nodes: ClueNode[]): void {
    setStorageItem(BROWSER_STORAGE_KEY, JSON.stringify(nodes));
  }

  async listNodes(): Promise<ClueNode[]> {
    return this.readAll();
  }

  async upsertNodes(nodes: ClueNode[]): Promise<void> {
    if (nodes.length === 0) {
      return;
    }

    const snapshot = this.readAll();
    const map = new Map(snapshot.map((node) => [node.id, node]));

    for (const node of nodes) {
      const normalizedId = String(canonicalShapeId(node.id));

      map.set(normalizedId, {
        ...node,
        id: normalizedId,
      });
    }

    this.writeAll(Array.from(map.values()));
  }

  async deleteNodes(ids: string[]): Promise<void> {
    if (ids.length === 0) {
      return;
    }

    const toDelete = new Set(ids.map((id) => String(canonicalShapeId(id))));
    const next = this.readAll().filter((node) => !toDelete.has(node.id));
    this.writeAll(next);
  }
}

class TauriNodeGateway implements NodeGateway {
  async listNodes(): Promise<ClueNode[]> {
    const nodes = await invoke<unknown>("get_nodes");
    return normalizeNodes(nodes);
  }

  async upsertNodes(nodes: ClueNode[]): Promise<void> {
    if (nodes.length === 0) {
      return;
    }

    const payload = nodes.map((node) => ({
      ...node,
      id: String(canonicalShapeId(node.id)),
    }));

    await invoke("upsert_nodes", { nodes: payload });
  }

  async deleteNodes(ids: string[]): Promise<void> {
    if (ids.length === 0) {
      return;
    }

    const payload = ids.map((id) => String(canonicalShapeId(id)));
    await invoke("delete_nodes", { ids: payload });
  }
}

export const runtimeMode: RuntimeMode = isTauriRuntime() ? "tauri" : "browser";
export const nodeGateway: NodeGateway = runtimeMode === "tauri" ? new TauriNodeGateway() : new BrowserNodeGateway();
