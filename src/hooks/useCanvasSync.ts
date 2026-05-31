import { useCallback, useEffect, useRef, useState } from "react";
import {
  createShapeId,
  toRichText,
  type Editor,
  type TLStoreEventInfo,
} from "tldraw";

import type { ClueNode } from "../domain/clueNode";
import { isTrackedShape, nodeToShape, shapeToNode } from "../domain/shapeMapper";
import { collectSyncDelta } from "../domain/storeChanges";
import { nodeGateway, runtimeMode } from "../infrastructure/nodeGateway";

const SYNC_DEBOUNCE_MS = 350;

export type SyncStatus = "hydrating" | "idle" | "saving" | "error";

function countTrackedShapes(editor: Editor): number {
  return editor.getCurrentPageShapes().filter((shape) => isTrackedShape(shape)).length;
}

export function useCanvasSync() {
  const editorRef = useRef<Editor | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);
  const hydrateGuardRef = useRef(true);
  const flushTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingUpsertsRef = useRef(new Map<string, ClueNode>());
  const pendingDeletesRef = useRef(new Set<string>());
  const quickNoteIndexRef = useRef(1);

  const [syncStatus, setSyncStatus] = useState<SyncStatus>("hydrating");
  const [syncError, setSyncError] = useState<string | null>(null);
  const [trackedCount, setTrackedCount] = useState(0);

  const flushPendingChanges = useCallback(async () => {
    flushTimeoutRef.current = null;

    const upserts = Array.from(pendingUpsertsRef.current.values());
    const deletes = Array.from(pendingDeletesRef.current.values());

    if (upserts.length === 0 && deletes.length === 0) {
      setSyncStatus("idle");
      return;
    }

    pendingUpsertsRef.current.clear();
    pendingDeletesRef.current.clear();

    try {
      if (upserts.length > 0) {
        await nodeGateway.upsertNodes(upserts);
      }

      if (deletes.length > 0) {
        await nodeGateway.deleteNodes(deletes);
      }

      setSyncStatus("idle");
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to synchronize nodes with persistence layer.";
      setSyncError(message);
      setSyncStatus("error");
    }
  }, []);

  const scheduleFlush = useCallback(() => {
    if (flushTimeoutRef.current !== null) {
      clearTimeout(flushTimeoutRef.current);
    }

    flushTimeoutRef.current = setTimeout(() => {
      void flushPendingChanges();
    }, SYNC_DEBOUNCE_MS);
  }, [flushPendingChanges]);

  const handleStoreEvent = useCallback(
    (editor: Editor, event: TLStoreEventInfo) => {
      if (hydrateGuardRef.current) {
        return;
      }

      const delta = collectSyncDelta(event);
      const hasChanges = delta.upserts.length > 0 || delta.deletedIds.length > 0;

      setTrackedCount(countTrackedShapes(editor));

      if (!hasChanges) {
        return;
      }

      for (const node of delta.upserts) {
        pendingDeletesRef.current.delete(node.id);
        pendingUpsertsRef.current.set(node.id, node);
      }

      for (const id of delta.deletedIds) {
        pendingUpsertsRef.current.delete(id);
        pendingDeletesRef.current.add(id);
      }

      setSyncStatus("saving");
      scheduleFlush();
    },
    [scheduleFlush],
  );

  const handleMount = useCallback(
    async (editor: Editor) => {
      editorRef.current = editor;
      hydrateGuardRef.current = true;
      setSyncStatus("hydrating");
      setSyncError(null);

      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }

      try {
        const nodes = await nodeGateway.listNodes();

        if (nodes.length > 0) {
          editor.run(() => {
            for (const node of nodes) {
              const shape = nodeToShape(node);

              if (editor.getShape(shape.id)) {
                editor.updateShape(shape);
              } else {
                editor.createShape(shape);
              }
            }
          });
        }

        setTrackedCount(countTrackedShapes(editor));
        setSyncStatus("idle");
      } catch (error) {
        const message = error instanceof Error ? error.message : "Failed to load persisted clues.";
        setSyncError(message);
        setSyncStatus("error");
      } finally {
        hydrateGuardRef.current = false;
      }

      unlistenRef.current = editor.store.listen(
        (event) => {
          handleStoreEvent(editor, event);
        },
        {
          source: "user",
          scope: "document",
        },
      );
    },
    [handleStoreEvent],
  );

  const createQuickNote = useCallback(() => {
    const editor = editorRef.current;

    if (!editor) {
      return;
    }

    const index = quickNoteIndexRef.current;
    quickNoteIndexRef.current += 1;
    const shapeId = createShapeId();

    editor.createShape({
      id: shapeId,
      type: "note",
      x: 120 + index * 24,
      y: 140 + index * 24,
      props: {
        richText: toRichText(`Clue ${index}`),
      },
    });

    const createdShape = editor.getShape(shapeId);
    const node = createdShape ? shapeToNode(createdShape) : null;

    if (node) {
      pendingDeletesRef.current.delete(node.id);
      pendingUpsertsRef.current.set(node.id, node);
      setTrackedCount(countTrackedShapes(editor));
      setSyncStatus("saving");
      scheduleFlush();
    }
  }, [scheduleFlush]);

  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
      }

      if (flushTimeoutRef.current !== null) {
        clearTimeout(flushTimeoutRef.current);
      }
    };
  }, []);

  return {
    handleMount,
    createQuickNote,
    syncStatus,
    syncError,
    trackedCount,
    runtimeMode,
  };
}
