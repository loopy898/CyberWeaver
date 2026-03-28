from __future__ import annotations

import json
import sys
from typing import Any, Dict, List

from sdk import ToolContext, TOOL_REGISTRY, register_tool


@register_tool("scan_port")
def scan_port(params: Dict[str, Any], graph_context: Dict[str, Any]) -> Dict[str, Any]:
    target_id = str(params.get("target_id", "")).strip()
    if not target_id:
        return {
            "message": "scan_port failed: missing target_id",
            "added_nodes": [],
            "added_edges": [],
            "tokens": ["缺少目标节点，无法执行扫描。"],
        }

    _ = ToolContext(params=params, graph_context=graph_context)

    discovered_ports: List[int] = [22, 80, 443]
    result_node_id = f"scan:{target_id}"
    summary = f"{target_id} 开放端口: {', '.join(str(port) for port in discovered_ports)}"

    return {
        "message": f"scan_port completed for {target_id}",
        "added_nodes": [
            {
                "id": result_node_id,
                "type": "note",
                "x": 520,
                "y": 220,
                "content": summary,
            }
        ],
        "added_edges": [
            {
                "source_id": target_id,
                "target_id": result_node_id,
                "relation": "scan_result",
                "properties": {"ports": discovered_ports},
            }
        ],
        "tokens": [
            f"观察目标节点 {target_id}",
            "调用扫描插件并收集端口开放信息",
            f"生成结论：{summary}",
        ],
    }


def main() -> int:
    if len(sys.argv) < 3 or sys.argv[1] != "--invoke":
        print(json.dumps({"error": "usage: scan_port.py --invoke <json>"}))
        return 1

    payload = json.loads(sys.argv[2])
    tool_name = payload.get("tool")
    params = payload.get("params", {})
    context = payload.get("context", {})

    tool = TOOL_REGISTRY.get(tool_name)
    if tool is None:
        print(json.dumps({"error": f"unknown tool: {tool_name}"}))
        return 1

    result = tool(params, context)
    print(json.dumps(result, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
