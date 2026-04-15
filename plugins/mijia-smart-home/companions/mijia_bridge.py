import json
import sys
import time
import types
from datetime import datetime, timedelta
from pathlib import Path
from urllib import parse

AUTH_BASE_DIR = (Path.home() / ".peekoo" / "mijia").resolve()


def _debug_log(message):
    print(f"[mijia_bridge] {message}", file=sys.stderr)


def _bootstrap_import_paths():
    script_dir = Path(__file__).resolve().parent
    plugin_dir = script_dir.parent
    vendor_candidates = [
        plugin_dir / "vendor",
        Path.home() / ".peekoo" / "mijia" / "vendor",
        Path.home() / ".peekoo" / "plugins" / "mijia-smart-home" / "vendor",
        Path.cwd() / "plugins" / "mijia-smart-home" / "vendor",
        Path("/tmp/mijia-api-inspect"),
    ]
    for p in vendor_candidates:
        if p.exists():
            sys.path.insert(0, str(p))

    try:
        import qrcode  # noqa: F401
    except ModuleNotFoundError:
        module = types.ModuleType("qrcode")
        class _DummyQRCode:
            def __init__(self, *args, **kwargs):
                pass
            def add_data(self, *args, **kwargs):
                pass
            def print_ascii(self, *args, **kwargs):
                pass
        module.QRCode = _DummyQRCode
        sys.modules["qrcode"] = module

_bootstrap_import_paths()


def emit(payload, code=0):
    sys.stdout.write(json.dumps(payload, ensure_ascii=False))
    sys.stdout.flush()
    raise SystemExit(code)


def _normalize_auth_path(raw):
    AUTH_BASE_DIR.mkdir(parents=True, exist_ok=True)
    if raw:
        p = Path(raw).expanduser()
        if not p.is_absolute():
            p = AUTH_BASE_DIR / p
    else:
        p = AUTH_BASE_DIR / "auth.json"
    if p.is_dir():
        p = p / "auth.json"
    p = p.resolve()
    if AUTH_BASE_DIR != p and AUTH_BASE_DIR not in p.parents:
        raise ValueError("auth_path must be inside ~/.peekoo/mijia")
    return p


def _build_room_index(homes):
    room_by_did = {}
    rooms = [{"id": "all", "name": "All Rooms", "home_id": "all"}]
    for home in homes:
        home_id = str(home.get("id", ""))
        for room in home.get("roomlist", []):
            room_id = str(room.get("id", ""))
            room_name = room.get("name", "Unnamed Room")
            rooms.append({"id": room_id, "name": room_name, "home_id": home_id})
            for did in room.get("dids", []) or []:
                room_by_did[str(did)] = {"room_id": room_id, "room_name": room_name, "home_id": home_id}
    rooms.append({"id": "shared", "name": "Shared Devices", "home_id": "shared"})
    return room_by_did, rooms


def _detect_toggle_property(props):
    for prop in props:
        name = str(prop.get("name", "")).strip().lower()
        rw = str(prop.get("rw", ""))
        ptype = str(prop.get("type", ""))
        if "w" not in rw:
            continue
        if name == "on":
            return prop
        if ptype == "bool" and ("switch" in name or "power" in name):
            return prop
    return None


def _safe_device_info(get_device_info, model, cache_dir):
    try:
        return get_device_info(model, cache_path=cache_dir)
    except Exception as err:
        _debug_log(f"get_device_info failed for model={model}: {err}")
        return {"properties": [], "actions": []}


def _auth_ready(api):
    try:
        return bool(api.available)
    except (AttributeError, TypeError, ValueError):
        return False


def _api_or_auth_required(payload):
    from mijiaAPI import mijiaAPI
    auth_path = _normalize_auth_path(payload.get("auth_path"))
    api = mijiaAPI(auth_data_path=str(auth_path))
    if not _auth_ready(api):
        emit({
            "success": False,
            "auth_required": True,
            "message": "Please sign in by scanning the QR code first",
            "auth_path": str(auth_path),
        })
    return api, auth_path


def action_status(payload):
    from mijiaAPI import mijiaAPI
    auth_path = _normalize_auth_path(payload.get("auth_path"))
    api = mijiaAPI(auth_data_path=str(auth_path))
    emit({
        "success": True,
        "authenticated": _auth_ready(api),
        "auth_path": str(auth_path),
    })


def action_login_start(payload):
    import requests
    from mijiaAPI import mijiaAPI

    auth_path = _normalize_auth_path(payload.get("auth_path"))
    pending_path = auth_path.parent / "login_pending.json"
    api = mijiaAPI(auth_data_path=str(auth_path))

    location_data = api._get_location()
    if location_data.get("code", -1) == 0:
        api._save_auth_data()
        api._init_session()
        if pending_path.exists():
            pending_path.unlink(missing_ok=True)
        emit({"success": True, "authenticated": True, "message": "Token is still valid. No need to scan again."})

    location_data.update({
        "theme": "",
        "bizDeviceType": "",
        "_hasLogo": "false",
        "_qrsize": "240",
        "_dc": str(int(time.time() * 1000)),
    })

    url = api.login_url + "?" + parse.urlencode(location_data)
    headers = {
        "User-Agent": api.user_agent,
        "Accept-Encoding": "gzip",
        "Content-Type": "application/x-www-form-urlencoded",
        "Connection": "keep-alive",
    }

    login_ret = requests.get(url, headers=headers, timeout=30)
    login_data = api._handle_ret(login_ret)

    pending_payload = {
        "lp": login_data["lp"],
        "headers": headers,
        "created_at": int(time.time()),
    }
    pending_path.parent.mkdir(parents=True, exist_ok=True)
    pending_path.write_text(json.dumps(pending_payload, ensure_ascii=False), encoding="utf-8")

    emit({
        "success": True,
        "authenticated": False,
        "needs_scan": True,
        "qr_url": login_data.get("qr"),
        "login_url": login_data.get("loginUrl"),
        "message": "Scan the QR code in Mijia app and confirm sign-in",
    })


def action_login_finish(payload):
    import requests
    from mijiaAPI import mijiaAPI

    auth_path = _normalize_auth_path(payload.get("auth_path"))
    pending_path = auth_path.parent / "login_pending.json"
    if not pending_path.exists():
        emit({"success": False, "message": "No pending login session found. Generate a QR code first."})

    api = mijiaAPI(auth_data_path=str(auth_path))
    pending = json.loads(pending_path.read_text(encoding="utf-8"))
    session = requests.Session()

    timeout_secs = int(payload.get("timeout_secs", 120))
    timeout_secs = 120 if timeout_secs <= 0 else min(timeout_secs, 300)

    try:
        lp_ret = session.get(pending["lp"], headers=pending["headers"], timeout=timeout_secs)
        lp_data = api._handle_ret(lp_ret)
    except requests.exceptions.Timeout:
        emit({"success": False, "pending": True, "message": "QR confirmation timed out. Please scan again."})

    auth_keys = ["psecurity", "nonce", "ssecurity", "passToken", "userId", "cUserId"]
    for key in auth_keys:
        api.auth_data[key] = lp_data[key]

    callback_url = lp_data["location"]
    session.get(callback_url, headers=pending["headers"], timeout=30)
    cookies = session.cookies.get_dict()
    api.auth_data.update(cookies)
    api.auth_data.update({
        "expireTime": int((datetime.now() + timedelta(days=30)).timestamp() * 1000),
    })

    api._save_auth_data()
    api._init_session()
    pending_path.unlink(missing_ok=True)

    emit({"success": True, "authenticated": True, "message": "Sign-in successful"})


def action_logout(payload):
    auth_path = _normalize_auth_path(payload.get("auth_path"))
    pending_path = auth_path.parent / "login_pending.json"
    removed = []

    if auth_path.exists():
        auth_path.unlink(missing_ok=True)
        removed.append(str(auth_path))

    if pending_path.exists():
        pending_path.unlink(missing_ok=True)
        removed.append(str(pending_path))

    emit({
        "success": True,
        "authenticated": False,
        "logged_out": True,
        "auth_path": str(auth_path),
        "removed": removed,
        "message": "Signed out successfully",
    })


def action_list_devices(payload):
    from mijiaAPI import get_device_info

    api, auth_path = _api_or_auth_required(payload)

    homes = api.get_homes_list()
    devices = api.get_devices_list() + api.get_shared_devices_list()

    home_map = {str(h.get("id")): h for h in homes}
    room_by_did, rooms = _build_room_index(homes)

    models = {str(d.get("model", "")) for d in devices if d.get("model")}
    model_toggle_map = {}
    for model in models:
        info = _safe_device_info(get_device_info, model, auth_path.parent)
        prop = _detect_toggle_property(info.get("properties", []))
        model_toggle_map[model] = prop

    prop_queries = []
    for device in devices:
        did = str(device.get("did", ""))
        model = str(device.get("model", ""))
        toggle_prop = model_toggle_map.get(model)
        if not toggle_prop:
            continue
        method = toggle_prop.get("method", {})
        query = {
            "did": did,
            "siid": method.get("siid"),
            "piid": method.get("piid"),
        }
        if query["siid"] is None or query["piid"] is None:
            continue
        prop_queries.append(query)

    values_by_key = {}
    if prop_queries:
        try:
            prop_results = api.get_devices_prop(prop_queries)
            for item in prop_results:
                key = f"{item.get('did')}:{item.get('siid')}:{item.get('piid')}"
                values_by_key[key] = item.get("value")
        except Exception as err:
            _debug_log(f"get_devices_prop failed in list_devices: {err}")

    home_filter = str(payload.get("home_id", "all"))
    room_filter = str(payload.get("room_id", "all"))

    result_devices = []
    for device in devices:
        did = str(device.get("did", ""))
        home_id = str(device.get("home_id", ""))
        model = str(device.get("model", ""))

        room_info = room_by_did.get(did, {
            "room_id": "shared" if home_id == "shared" else "unknown",
            "room_name": "Shared Devices" if home_id == "shared" else "Unassigned",
            "home_id": home_id,
        })

        if home_filter not in ("", "all") and home_id != home_filter:
            continue
        if room_filter not in ("", "all") and room_info["room_id"] != room_filter:
            continue

        toggle_prop = model_toggle_map.get(model)
        quick_toggle = {"supported": False}
        if toggle_prop:
            method = toggle_prop.get("method", {})
            siid = method.get("siid")
            piid = method.get("piid")
            value = values_by_key.get(f"{did}:{siid}:{piid}")
            quick_toggle = {
                "supported": True,
                "prop_name": toggle_prop.get("name"),
                "siid": siid,
                "piid": piid,
                "current": value,
            }

        result_devices.append({
            "did": did,
            "name": device.get("name", did),
            "model": model,
            "is_online": bool(device.get("isOnline", False)),
            "home_id": home_id,
            "home_name": home_map.get(home_id, {}).get("name", "Shared") if home_id != "shared" else "Shared",
            "room_id": room_info.get("room_id"),
            "room_name": room_info.get("room_name"),
            "icon": device.get("icon"),
            "quick_toggle": quick_toggle,
            "raw": device,
        })

    homes_payload = [{"id": "all", "name": "All Homes"}] + [
        {"id": str(h.get("id", "")), "name": h.get("name", "Unnamed Home")} for h in homes
    ]

    emit({
        "success": True,
        "authenticated": True,
        "homes": homes_payload,
        "rooms": rooms,
        "devices": result_devices,
    })


def _find_device(api, did):
    devices = api.get_devices_list() + api.get_shared_devices_list()
    for device in devices:
        if str(device.get("did")) == str(did):
            return device
    return None


def _parse_stat_value(raw):
    import ast
    if isinstance(raw, (int, float)):
        return raw
    if isinstance(raw, str):
        s = raw.strip()
        if not s:
            return None
        try:
            parsed = json.loads(s)
            if isinstance(parsed, list) and parsed:
                return parsed[0]
            if isinstance(parsed, (int, float)):
                return parsed
        except (json.JSONDecodeError, TypeError, ValueError):
            # Parse failure is expected for non-JSON strings; try literal_eval next.
            _debug_log(f"json parse failed for stat value: {s!r}")
        try:
            parsed = ast.literal_eval(s)
            if isinstance(parsed, list) and parsed:
                return parsed[0]
            if isinstance(parsed, (int, float)):
                return parsed
        except (SyntaxError, ValueError):
            # Parse failure is expected for non-literal strings; return None.
            _debug_log(f"literal_eval parse failed for stat value: {s!r}")
    return None


def _read_stat_metric(api, did, key, data_types, time_start, time_end):
    for data_type in data_types:
        try:
            ret = api.get_statistics({
                "did": did,
                "key": key,
                "data_type": data_type,
                "limit": 1,
                "time_start": time_start,
                "time_end": time_end,
            })
            rows = ret.get("result") if isinstance(ret, dict) else ret
            if isinstance(rows, list) and rows:
                value = _parse_stat_value(rows[0].get("value"))
                if value is not None:
                    return value
        except Exception as err:
            _debug_log(f"get_statistics failed for key={key}, data_type={data_type}: {err}")
            continue
    return None


def _energy_scale_from_prop(prop):
    name = str(prop.get("name", "")).lower()
    desc = str(prop.get("description", "")).lower()
    unit = str(prop.get("unit", "")).lower()
    text = f"{name} {desc} {unit}"
    # Many plug models use "0.001kwh" as the base unit.
    if "0.001kwh" in text:
        return 0.001
    return 1.0


def action_toggle_device(payload):
    from mijiaAPI import get_device_info

    api, auth_path = _api_or_auth_required(payload)
    did = str(payload.get("did", "")).strip()
    if not did:
        emit({"success": False, "message": "did is required"})

    device = _find_device(api, did)
    if not device:
        emit({"success": False, "message": "Device not found"})

    info = _safe_device_info(get_device_info, str(device.get("model", "")), auth_path.parent)
    toggle_prop = _detect_toggle_property(info.get("properties", []))
    if not toggle_prop:
        emit({"success": False, "message": "This device does not support quick toggle"})

    method = toggle_prop.get("method", {})
    siid = method.get("siid")
    piid = method.get("piid")
    if siid is None or piid is None:
        emit({"success": False, "message": "Toggle property metadata is incomplete"})

    target = payload.get("value", None)
    if target is None:
        curr = api.get_devices_prop({"did": did, "siid": siid, "piid": piid})
        target = not bool(curr.get("value"))
    else:
        target = bool(target)

    ret = api.set_devices_prop({"did": did, "siid": siid, "piid": piid, "value": target})
    emit({"success": True, "result": ret, "value": target})


def action_device_detail(payload):
    from mijiaAPI import get_device_info

    api, auth_path = _api_or_auth_required(payload)
    did = str(payload.get("did", "")).strip()
    if not did:
        emit({"success": False, "message": "did is required"})

    device = _find_device(api, did)
    if not device:
        emit({"success": False, "message": "Device not found"})

    model = str(device.get("model", ""))
    info = _safe_device_info(get_device_info, model, auth_path.parent)
    properties = info.get("properties", [])
    actions = info.get("actions", [])

    readable_methods = []
    for prop in properties:
        rw = str(prop.get("rw", ""))
        method = prop.get("method", {})
        if "r" in rw and method.get("siid") is not None and method.get("piid") is not None:
            readable_methods.append({"did": did, "siid": method["siid"], "piid": method["piid"]})

    current_values = {}
    if readable_methods:
        try:
            prop_results = api.get_devices_prop(readable_methods)
            for item in prop_results:
                current_values[f"{item.get('siid')}:{item.get('piid')}"] = item.get("value")
        except Exception as err:
            _debug_log(f"get_devices_prop failed in device_detail: {err}")

    normalized_props = []
    for prop in properties:
        method = prop.get("method", {})
        key = f"{method.get('siid')}:{method.get('piid')}"
        normalized_props.append({
            "name": prop.get("name"),
            "description": prop.get("description"),
            "type": prop.get("type"),
            "rw": prop.get("rw"),
            "unit": prop.get("unit"),
            "range": prop.get("range"),
            "value_list": prop.get("value-list"),
            "method": method,
            "current_value": current_values.get(key),
        })

    normalized_actions = []
    for action in actions:
        normalized_actions.append({
            "name": action.get("name"),
            "description": action.get("description"),
            "method": action.get("method"),
        })

    # Energy summary: today usage / monthly usage / current power.
    current_power = None
    for prop in normalized_props:
        pname = str(prop.get("name", "")).lower()
        if pname in ("power", "electric-power"):
            current_power = prop.get("current_value")
            if current_power is not None:
                break

    consumption_props = []
    stat_queries = []
    for prop in normalized_props:
        pname = str(prop.get("name", "")).lower()
        is_consumption = any(k in pname for k in ("power-consumption", "powercost", "energy"))
        if is_consumption:
            consumption_props.append(prop)
            method = prop.get("method", {})
            siid = method.get("siid")
            piid = method.get("piid")
            if siid is not None and piid is not None:
                stat_queries.append({
                    "key": f"{siid}.{piid}",
                    "scale": _energy_scale_from_prop(prop),
                })
    stat_queries.extend([
        {"key": "powerCost", "scale": 1.0},
        {"key": "power-consumption", "scale": 0.001},
    ])
    seen = set()
    uniq_queries = []
    for item in stat_queries:
        key = item["key"]
        if key in seen:
            continue
        seen.add(key)
        uniq_queries.append(item)
    stat_queries = uniq_queries

    now_ts = int(time.time())
    day_start = now_ts - 2 * 24 * 3600
    month_start = now_ts - 40 * 24 * 3600
    day_types = ["stat_day_v3", "stat_day"]
    month_types = ["stat_month_v3", "stat_month"]

    # For most smart plugs, current "power-consumption/powerCost" is today's usage.
    today_usage = None
    for prop in consumption_props:
        curr = prop.get("current_value")
        if curr is None:
            continue
        try:
            today_usage = float(curr) * _energy_scale_from_prop(prop)
            break
        except (TypeError, ValueError):
            continue

    month_usage = None
    for item in stat_queries:
        key = item["key"]
        scale = float(item.get("scale", 1.0) or 1.0)
        # Keep statistics API as fallback for today if direct property is unavailable.
        if today_usage is None:
            raw_today = _read_stat_metric(api, did, key, day_types, day_start, now_ts)
            if raw_today is not None:
                today_usage = float(raw_today) * scale
        if month_usage is None:
            raw_month = _read_stat_metric(api, did, key, month_types, month_start, now_ts)
            if raw_month is not None:
                month_usage = float(raw_month) * scale
        if today_usage is not None and month_usage is not None:
            break

    emit({
        "success": True,
        "device": {
            "did": did,
            "name": device.get("name", did),
            "model": model,
            "is_online": bool(device.get("isOnline", False)),
            "raw": device,
        },
        "properties": normalized_props,
        "actions": normalized_actions,
        "energy_summary": {
            "today": today_usage,
            "month": month_usage,
            "current_power": current_power,
        },
    })


def action_set_property(payload):
    from mijiaAPI import mijiaDevice

    api, _ = _api_or_auth_required(payload)
    did = str(payload.get("did", "")).strip()
    prop_name = str(payload.get("prop_name", "")).strip()
    value = payload.get("value", None)

    if not did or not prop_name:
        emit({"success": False, "message": "did and prop_name are required"})

    device = mijiaDevice(api, did=did)
    device.set(prop_name, value)
    latest = device.get(prop_name)
    emit({"success": True, "value": latest, "message": "Property updated successfully"})


def action_run_action(payload):
    from mijiaAPI import mijiaDevice

    api, _ = _api_or_auth_required(payload)
    did = str(payload.get("did", "")).strip()
    action_name = str(payload.get("action_name", "")).strip()
    value = payload.get("value", None)

    if not did or not action_name:
        emit({"success": False, "message": "did and action_name are required"})

    device = mijiaDevice(api, did=did)
    if value is None:
        device.run_action(action_name)
    else:
        device.run_action(action_name, value=value)

    emit({"success": True, "message": "Action executed successfully"})


def main():
    if len(sys.argv) < 3:
        emit({"success": False, "message": "Missing arguments"}, 1)

    action = sys.argv[1]
    payload_text = sys.argv[2]
    try:
        payload = json.loads(payload_text)
    except (json.JSONDecodeError, TypeError, ValueError):
        payload = {}

    try:
        from mijiaAPI.logger import logger
        import logging

        logger.handlers = []
        logger.setLevel(logging.CRITICAL)
    except ModuleNotFoundError:
        # Logger module is optional; continue with default logging behavior.
        pass
    except Exception as err:
        _debug_log(f"failed to mute mijiaAPI logger: {err}")

    actions = {
        "status": action_status,
        "login_start": action_login_start,
        "login_finish": action_login_finish,
        "logout": action_logout,
        "list_devices": action_list_devices,
        "toggle_device": action_toggle_device,
        "device_detail": action_device_detail,
        "set_property": action_set_property,
        "run_action": action_run_action,
    }

    if action not in actions:
        emit({"success": False, "message": f"Unsupported action: {action}"}, 1)

    actions[action](payload)


if __name__ == "__main__":
    try:
        main()
    except ModuleNotFoundError as err:
        msg = str(err)
        if "mijiaAPI" in msg:
            emit(
                {
                    "success": False,
                    "message": "Python package mijiaAPI not found in runtime. Repackage the plugin runtime or install with: pip install mijiaAPI",
                    "code": "mijia_api_missing",
                },
                1,
            )
        if "requests" in msg:
            emit(
                {
                    "success": False,
                    "message": "Python package requests not found in runtime. Repackage the plugin runtime or install with: pip install requests",
                    "code": "requests_missing",
                },
                1,
            )
        emit({"success": False, "message": msg}, 1)
    except Exception as err:
        emit({"success": False, "message": str(err)}, 1)
