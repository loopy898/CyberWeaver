from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable, Dict


ToolFn = Callable[[Dict[str, Any], Dict[str, Any]], Dict[str, Any]]
TOOL_REGISTRY: Dict[str, ToolFn] = {}


def register_tool(name: str):
    def decorator(func: ToolFn) -> ToolFn:
        TOOL_REGISTRY[name] = func
        return func

    return decorator


@dataclass
class ToolContext:
    params: Dict[str, Any]
    graph_context: Dict[str, Any]
