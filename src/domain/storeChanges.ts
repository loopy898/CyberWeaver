import { isShape, type TLRecord, type TLStoreEventInfo } from "tldraw";

import type { ClueNode } from "./clueNode";
import { isTrackedShapeType } from "./clueNode";
import { shapeToNode } from "./shapeMapper";

export interface SyncDelta {
  upserts: ClueNode[];
  deletedIds: string[];
}

function collectUpsert(record: TLRecord, upsertsById: Map<string, ClueNode>): void {
  const node = shapeToNode(record);

  if (node) {
    upsertsById.set(node.id, node);
  }
}

export function collectSyncDelta(event: TLStoreEventInfo): SyncDelta {
  const upsertsById = new Map<string, ClueNode>();
  const deletedIds = new Set<string>();

  for (const record of Object.values(event.changes.added)) {
    collectUpsert(record, upsertsById);
  }

  for (const update of Object.values(event.changes.updated)) {
    const nextRecord = update[1];
    collectUpsert(nextRecord, upsertsById);
  }

  for (const record of Object.values(event.changes.removed)) {
    if (isShape(record) && isTrackedShapeType(record.type)) {
      upsertsById.delete(record.id);
      deletedIds.add(record.id);
    }
  }

  return {
    upserts: Array.from(upsertsById.values()),
    deletedIds: Array.from(deletedIds.values()),
  };
}
