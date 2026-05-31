from __future__ import annotations

import hashlib
import json
from typing import Any, Dict, Tuple
from urllib.parse import urlencode
from urllib.request import Request, urlopen

from sdk import ToolContext, register_tool


_NODE_X = 660
_NODE_Y = 280


def _stable_node_id(prefix: str, content: str) -> str:
    digest = hashlib.sha1(content.encode("utf-8")).hexdigest()[:12]
    return f"{prefix}:{digest}"


def _parse_coordinates(params: Dict[str, Any]) -> Tuple[float, float]:
    if "latitude" not in params or "longitude" not in params:
        raise ValueError("missing latitude or longitude")

    latitude = float(params["latitude"])
    longitude = float(params["longitude"])

    if latitude < -90 or latitude > 90:
        raise ValueError("latitude out of range")
    if longitude < -180 or longitude > 180:
        raise ValueError("longitude out of range")
    return latitude, longitude


def _query_reverse_geocode(latitude: float, longitude: float) -> Dict[str, Any]:
    query = urlencode(
        {
            "format": "jsonv2",
            "lat": f"{latitude:.8f}",
            "lon": f"{longitude:.8f}",
            "addressdetails": 1,
        }
    )
    request = Request(
        f"https://nominatim.openstreetmap.org/reverse?{query}",
        headers={"User-Agent": "CyberWeaver/1.0 (tool reverse_geocode)"},
    )
    with urlopen(request, timeout=10) as response:
        payload = response.read().decode("utf-8")
    return json.loads(payload)


def _build_tokens(payload: Dict[str, Any], latitude: float, longitude: float) -> list[str]:
    address = payload.get("address", {}) if isinstance(payload.get("address"), dict) else {}
    display_name = str(payload.get("display_name", "")).strip()
    country = str(address.get("country", "")).strip()
    state = str(address.get("state", "")).strip()
    city = str(address.get("city") or address.get("town") or address.get("county") or "").strip()
    district = str(address.get("suburb") or address.get("city_district") or "").strip()

    tokens = [
        f"query: latitude={latitude:.8f}, longitude={longitude:.8f}",
        f"display_name: {display_name or 'N/A'}",
        f"country: {country or 'N/A'}",
        f"state: {state or 'N/A'}",
        f"city_or_county: {city or 'N/A'}",
        f"district: {district or 'N/A'}",
    ]
    return tokens


@register_tool("reverse_geocode")
def reverse_geocode(params: Dict[str, Any], graph_context: Dict[str, Any]) -> Dict[str, Any]:
    _ = ToolContext(params=params, graph_context=graph_context)

    try:
        latitude, longitude = _parse_coordinates(params)
    except Exception as error:
        return {
            "message": f"reverse_geocode failed: {error}",
            "added_nodes": [],
            "added_edges": [],
            "tokens": ["经纬度参数无效，无法查询位置。"],
        }

    try:
        mock_response = params.get("mock_response")
        if isinstance(mock_response, dict):
            payload = mock_response
        else:
            payload = _query_reverse_geocode(latitude, longitude)
    except Exception as error:
        return {
            "message": f"reverse_geocode failed: {error}",
            "added_nodes": [],
            "added_edges": [],
            "tokens": ["逆地理编码请求失败。"],
        }

    display_name = str(payload.get("display_name", "")).strip()
    if not display_name:
        return {
            "message": "reverse_geocode failed: empty provider result",
            "added_nodes": [],
            "added_edges": [],
            "tokens": ["服务返回为空，未获得可用地址。"],
        }

    tokens = _build_tokens(payload, latitude, longitude)
    content = "Reverse geocode result\n" + "\n".join(tokens)
    node_id = _stable_node_id("geo", content)
    return {
        "message": f"reverse_geocode completed for ({latitude:.6f}, {longitude:.6f})",
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
        "tokens": tokens,
    }
