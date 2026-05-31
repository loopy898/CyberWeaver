import {
  createShapeId,
  isShape,
  toRichText,
  type TLRecord,
  type TLShape,
  type TLShapeId,
  type TLShapePartial,
} from "tldraw";

import type { ClueNode } from "./clueNode";
import { isTrackedShapeType } from "./clueNode";
import { richTextToPlainText } from "./richText";

const DEFAULT_GEO_WIDTH = 200;
const DEFAULT_GEO_HEIGHT = 120;
const MIN_TEXT_WIDTH = 32;

export function canonicalShapeId(rawId: string): TLShapeId {
  const trimmed = rawId.trim();

  if (!trimmed) {
    return createShapeId();
  }

  const withoutPrefix = trimmed.startsWith("shape:") ? trimmed.slice("shape:".length) : trimmed;
  return createShapeId(withoutPrefix);
}

export function isTrackedShape(shape: TLShape): boolean {
  return isTrackedShapeType(shape.type);
}

export function nodeToShape(node: ClueNode): TLShapePartial {
  const id = canonicalShapeId(node.id);

  switch (node.type) {
    case "geo": {
      const width = Math.max(node.width ?? DEFAULT_GEO_WIDTH, 1);
      const height = Math.max(node.height ?? DEFAULT_GEO_HEIGHT, 1);

      return {
        id,
        type: "geo",
        x: node.x,
        y: node.y,
        props: {
          geo: "rectangle",
          w: width,
          h: height,
          richText: toRichText(node.content),
        },
      };
    }

    case "text": {
      const width = Math.max(node.width ?? MIN_TEXT_WIDTH, MIN_TEXT_WIDTH);

      return {
        id,
        type: "text",
        x: node.x,
        y: node.y,
        props: {
          richText: toRichText(node.content),
          w: width,
          autoSize: false,
        },
      };
    }

    case "note":
      return {
        id,
        type: "note",
        x: node.x,
        y: node.y,
        props: {
          richText: toRichText(node.content),
        },
      };
  }
}

function asRecordShape(record: TLRecord): TLShape | null {
  if (!isShape(record)) {
    return null;
  }

  return record;
}

export function shapeToNode(record: TLRecord): ClueNode | null {
  const shape = asRecordShape(record);

  if (!shape || !isTrackedShapeType(shape.type)) {
    return null;
  }

  if (shape.type === "geo") {
    const props = shape.props as { richText?: unknown; w?: number; h?: number };

    return {
      id: shape.id,
      type: "geo",
      x: shape.x,
      y: shape.y,
      content: richTextToPlainText(props.richText),
      width: typeof props.w === "number" ? props.w : DEFAULT_GEO_WIDTH,
      height: typeof props.h === "number" ? props.h : DEFAULT_GEO_HEIGHT,
    };
  }

  if (shape.type === "text") {
    const props = shape.props as { richText?: unknown; w?: number };

    return {
      id: shape.id,
      type: "text",
      x: shape.x,
      y: shape.y,
      content: richTextToPlainText(props.richText),
      width: typeof props.w === "number" ? props.w : MIN_TEXT_WIDTH,
      height: null,
    };
  }

  const props = shape.props as { richText?: unknown };

  return {
    id: shape.id,
    type: "note",
    x: shape.x,
    y: shape.y,
    content: richTextToPlainText(props.richText),
    width: null,
    height: null,
  };
}
