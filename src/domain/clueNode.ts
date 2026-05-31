export const TRACKED_SHAPE_TYPES = ["geo", "text", "note"] as const;

export type TrackedShapeType = (typeof TRACKED_SHAPE_TYPES)[number];

export interface ClueNode {
  id: string;
  type: TrackedShapeType;
  x: number;
  y: number;
  content: string;
  width: number | null;
  height: number | null;
}

export function isTrackedShapeType(value: string): value is TrackedShapeType {
  return TRACKED_SHAPE_TYPES.includes(value as TrackedShapeType);
}

export function asFiniteNumber(value: unknown): number | null {
  if (typeof value !== "number" || Number.isNaN(value) || !Number.isFinite(value)) {
    return null;
  }

  return value;
}

export function normalizeNode(raw: unknown): ClueNode | null {
  if (!raw || typeof raw !== "object") {
    return null;
  }

  const candidate = raw as Record<string, unknown>;

  if (typeof candidate.id !== "string") {
    return null;
  }

  if (typeof candidate.type !== "string" || !isTrackedShapeType(candidate.type)) {
    return null;
  }

  const x = asFiniteNumber(candidate.x);
  const y = asFiniteNumber(candidate.y);

  if (x === null || y === null) {
    return null;
  }

  const content = typeof candidate.content === "string" ? candidate.content : "";

  return {
    id: candidate.id,
    type: candidate.type,
    x,
    y,
    content,
    width: asFiniteNumber(candidate.width),
    height: asFiniteNumber(candidate.height),
  };
}
