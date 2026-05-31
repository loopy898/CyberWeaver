import { describe, expect, it } from "vitest";
import { toRichText, type TLStoreEventInfo } from "tldraw";

import { collectSyncDelta } from "./storeChanges";

function createNote(id: string, content: string) {
  return {
    id,
    typeName: "shape",
    type: "note",
    x: 10,
    y: 20,
    props: {
      richText: toRichText(content),
    },
  };
}

describe("collectSyncDelta", () => {
  it("converts added and updated records into upsert operations", () => {
    const previous = createNote("shape:note-1", "first");
    const updated = createNote("shape:note-1", "second");

    const event = {
      changes: {
        added: {},
        updated: {
          [updated.id]: [previous, updated],
        },
        removed: {},
      },
      source: "user",
    } as unknown as TLStoreEventInfo;

    const delta = collectSyncDelta(event);

    expect(delta.deletedIds).toEqual([]);
    expect(delta.upserts).toEqual([
      {
        id: "shape:note-1",
        type: "note",
        x: 10,
        y: 20,
        content: "second",
        width: null,
        height: null,
      },
    ]);
  });

  it("keeps delete operations when a tracked shape is removed", () => {
    const deleted = createNote("shape:note-2", "temporary");

    const event = {
      changes: {
        added: {},
        updated: {},
        removed: {
          [deleted.id]: deleted,
        },
      },
      source: "user",
    } as unknown as TLStoreEventInfo;

    const delta = collectSyncDelta(event);

    expect(delta.upserts).toEqual([]);
    expect(delta.deletedIds).toEqual(["shape:note-2"]);
  });
});
