from __future__ import annotations

import hashlib
from datetime import datetime, timedelta, timezone
from typing import Any, Dict, List, Tuple

from sdk import ToolContext, register_tool


_COCOA_EPOCH = datetime(2001, 1, 1, tzinfo=timezone.utc)
_UNIX_EPOCH = datetime(1970, 1, 1, tzinfo=timezone.utc)
_NODE_X = 620
_NODE_Y = 220


def _stable_node_id(prefix: str, content: str) -> str:
    digest = hashlib.sha1(content.encode("utf-8")).hexdigest()[:12]
    return f"{prefix}:{digest}"


def _parse_numeric_value(raw: Any) -> float:
    if raw is None:
        raise ValueError("missing value")
    if isinstance(raw, (int, float)):
        return float(raw)

    text = str(raw).strip()
    if not text:
        raise ValueError("missing value")
    return float(text)


def _to_datetime(label: str, value: float) -> datetime:
    if label == "unix_seconds":
        return _UNIX_EPOCH + timedelta(seconds=value)
    if label == "unix_milliseconds":
        return _UNIX_EPOCH + timedelta(milliseconds=value)
    if label == "unix_microseconds":
        return _UNIX_EPOCH + timedelta(microseconds=value)
    if label == "unix_nanoseconds":
        return _UNIX_EPOCH + timedelta(seconds=value / 1_000_000_000.0)
    if label == "apple_cocoa_seconds":
        return _COCOA_EPOCH + timedelta(seconds=value)
    raise ValueError(f"unknown label: {label}")


def _build_line(label: str, value: float) -> Tuple[str, datetime]:
    dt_utc = _to_datetime(label, value)
    try:
        dt_local = dt_utc.astimezone().isoformat()
    except Exception:
        # Windows may fail local conversion for very old timestamps.
        dt_local = "N/A"
    line = (
        f"{label}: value={value:g} | "
        f"utc={dt_utc.isoformat()} | "
        f"rfc3339={dt_utc.isoformat()} | "
        f"local={dt_local}"
    )
    return line, dt_utc


@register_tool("timestamp_convert")
def timestamp_convert(params: Dict[str, Any], graph_context: Dict[str, Any]) -> Dict[str, Any]:
    _ = ToolContext(params=params, graph_context=graph_context)

    try:
        value = _parse_numeric_value(params.get("value"))
    except ValueError as error:
        reason = str(error)
        return {
            "message": f"timestamp_convert failed: {reason}",
            "added_nodes": [],
            "added_edges": [],
            "tokens": [
                "无法解析输入时间戳。",
                "请提供可转换为数字的 value。",
            ],
        }
    except Exception as error:  # defensive guard
        return {
            "message": f"timestamp_convert failed: {error}",
            "added_nodes": [],
            "added_edges": [],
            "tokens": ["时间戳转换出现异常。"],
        }

    labels = [
        "unix_seconds",
        "unix_milliseconds",
        "unix_microseconds",
        "unix_nanoseconds",
        "apple_cocoa_seconds",
    ]

    lines: List[str] = []
    valid_dates: List[datetime] = []
    for label in labels:
        try:
            line, dt_utc = _build_line(label, value)
            lines.append(line)
            valid_dates.append(dt_utc)
        except Exception as error:
            lines.append(f"{label}: invalid ({error})")

    if not valid_dates:
        return {
            "message": "timestamp_convert failed: no valid interpretation",
            "added_nodes": [],
            "added_edges": [],
            "tokens": lines,
        }

    summary = f"timestamp_convert completed for value={value:g}"
    content = "Timestamp conversion results\n" + "\n".join(lines)
    node_id = _stable_node_id("timestamp", content)
    return {
        "message": summary,
        "added_nodes": [
            {
                "id": node_id,
                "type": "note",
                "x": _NODE_X,
                "y": _NODE_Y,
                "content": content,
            }
        ],
        "added_edges": [],
        "tokens": lines,
    }
