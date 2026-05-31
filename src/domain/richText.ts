interface RichTextLikeNode {
  type?: string;
  text?: string;
  content?: RichTextLikeNode[];
}

function flattenNode(node: RichTextLikeNode): string {
  if (typeof node.text === "string") {
    return node.text;
  }

  if (!Array.isArray(node.content)) {
    return "";
  }

  const separator = node.type === "paragraph" ? "" : "";
  return node.content.map(flattenNode).join(separator);
}

export function richTextToPlainText(richText: unknown): string {
  if (!richText || typeof richText !== "object") {
    return "";
  }

  const candidate = richText as RichTextLikeNode;

  if (!Array.isArray(candidate.content)) {
    return "";
  }

  const lines = candidate.content
    .map((node) => flattenNode(node).trim())
    .filter((line) => line.length > 0);

  return lines.join("\n");
}
