import { describe, expect, it } from "vitest";
import { toRichText } from "tldraw";
import type { TLRecord } from "tldraw";

import type { ClueNode } from "./clueNode";
import { canonicalShapeId, nodeToShape, shapeToNode } from "./shapeMapper";

describe("shapeMapper", () => {
  it("normalizes shape ids without duplicating the shape prefix", () => {
    expect(canonicalShapeId("shape:alpha")).toBe("shape:alpha");
    expect(canonicalShapeId("alpha")).toBe("shape:alpha");
  });

  it("maps persisted geo nodes to valid geo shapes", () => {
    const node: ClueNode = {
      id: "investigation-1",
      type: "geo",
      x: 64,
      y: 32,
      content: "entry",
      width: 280,
      height: 140,
    };

    const shape = nodeToShape(node);

    expect(shape.id).toBe("shape:investigation-1");
    expect(shape.type).toBe("geo");
    expect(shape.props).toMatchObject({
      geo: "rectangle",
      w: 280,
      h: 140,
    });
  });

  it("extracts plain text and geometry metadata from persisted shapes", () => {
    const record = {
      id: "shape:case-note",
      typeName: "shape",
      type: "text",
      x: 120,
      y: 80,
      props: {
        richText: toRichText("host compromised"),
        w: 360,
      },
    } as const;

    const node = shapeToNode(record as unknown as TLRecord);

    expect(node).toEqual({
      id: "shape:case-note",
      type: "text",
      x: 120,
      y: 80,
      content: "host compromised",
      width: 360,
      height: null,
    });
  });
});
