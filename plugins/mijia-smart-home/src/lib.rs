#![no_main]

mod api;
mod crypto;
mod device;
mod error;

use peekoo_plugin_sdk::prelude::*;
use serde_json::{json, Value};

use api::MijiaApi;

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.into())
}

#[plugin_fn]
pub fn plugin_shutdown(_: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.into())
}

#[derive(Deserialize)]
struct MijiaBridgeInput {
    action: String,
    #[serde(default)]
    payload: Value,
}

fn ok_response(data: Value) -> FnResult<String> {
    Ok(data.to_string())
}

fn error_response(message: &str) -> FnResult<String> {
    Ok(json!({"success": false, "message": message}).to_string())
}

fn error_response_code(message: &str, code: &str) -> FnResult<String> {
    Ok(json!({"success": false, "message": message, "code": code}).to_string())
}

#[plugin_fn]
pub fn tool_mijia_bridge(Json(input): Json<MijiaBridgeInput>) -> FnResult<String> {
    let action = input.action.trim();
    if action.is_empty() {
        return error_response("action is required");
    }

    match action {
        "status" => action_status(&input.payload),
        "login_start" => action_login_start(&input.payload),
        "login_finish" => action_login_finish(&input.payload),
        "logout" => action_logout(&input.payload),
        "list_devices" => action_list_devices(&input.payload),
        "toggle_device" => action_toggle_device(&input.payload),
        "device_detail" => action_device_detail(&input.payload),
        "set_property" => action_set_property(&input.payload),
        "run_action" => action_run_action(&input.payload),
        _ => error_response(&format!("Unsupported action: {action}")),
    }
}

fn action_status(_payload: &Value) -> FnResult<String> {
    let api = MijiaApi::load().map_err(|e| Error::msg(e.to_string()))?;
    ok_response(json!({
        "success": true,
        "authenticated": api.is_authenticated(),
        "auth_path": api.auth_path_display(),
    }))
}

fn action_login_start(_payload: &Value) -> FnResult<String> {
    let mut api = MijiaApi::load().map_err(|e| Error::msg(e.to_string()))?;
    match api.login_start() {
        Ok(result) => ok_response(result),
        Err(e) => error_response(&e.to_string()),
    }
}

fn action_login_finish(payload: &Value) -> FnResult<String> {
    let timeout = payload["timeout_secs"].as_i64().unwrap_or(120);
    let timeout = if timeout <= 0 { 120 } else { timeout.min(300) };

    let mut api = MijiaApi::load().map_err(|e| Error::msg(e.to_string()))?;
    match api.login_finish(timeout) {
        Ok(result) => ok_response(result),
        Err(e) => error_response(&e.to_string()),
    }
}

fn action_logout(_payload: &Value) -> FnResult<String> {
    match MijiaApi::logout() {
        Ok(result) => ok_response(result),
        Err(e) => error_response(&e.to_string()),
    }
}

fn action_list_devices(payload: &Value) -> FnResult<String> {
    let api = MijiaApi::load().map_err(|e| Error::msg(e.to_string()))?;
    if !api.is_authenticated() {
        return error_response_code(
            "Please sign in by scanning the QR code first",
            "auth_required",
        );
    }

    let homes = match api.get_homes_list() {
        Ok(h) => h,
        Err(e) => return error_response(&e.to_string()),
    };

    let all_devices = match api.get_all_devices() {
        Ok(d) => d,
        Err(e) => return error_response(&e.to_string()),
    };

    let shared_devices = match api.get_shared_devices_list() {
        Ok(d) => d,
        Err(e) => return error_response(&e.to_string()),
    };

    let home_map: std::collections::HashMap<String, Value> = homes
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|h| {
                    let id = h["id"].to_string().trim_matches('"').to_string();
                    Some((id, h.clone()))
                })
                .collect()
        })
        .unwrap_or_default();

    let (room_by_did, rooms) = build_room_index(&homes);

    // Merge all devices
    let mut devices: Vec<Value> = Vec::new();
    if let Some(arr) = all_devices.as_array() {
        devices.extend(arr.clone());
    }
    if let Some(arr) = shared_devices.as_array() {
        devices.extend(arr.clone());
    }

    // Fetch toggle properties for each model
    let models: std::collections::HashSet<String> = devices
        .iter()
        .filter_map(|d| d["model"].as_str().map(|s| s.to_string()))
        .collect();

    let mut model_toggle_map: std::collections::HashMap<String, Value> =
        std::collections::HashMap::new();
    for model in &models {
        match device::fetch_device_info(model) {
            Ok(info) => {
                let toggle = find_toggle_property(&info.properties);
                if let Some(prop) = toggle {
                    model_toggle_map.insert(
                        model.clone(),
                        serde_json::to_value(&prop).unwrap_or(Value::Null),
                    );
                }
            }
            Err(e) => {
                peekoo::log::debug(&format!("device info failed for {model}: {e}"));
            }
        }
    }

    // Batch query toggle state
    let mut prop_queries = Vec::new();
    for dev in &devices {
        let did = dev["did"].to_string().trim_matches('"').to_string();
        let model = dev["model"].as_str().unwrap_or("");
        if let Some(toggle_prop) = model_toggle_map.get(model) {
            let siid = toggle_prop["method"]["siid"].as_i64();
            let piid = toggle_prop["method"]["piid"].as_i64();
            if let (Some(s), Some(p)) = (siid, piid) {
                prop_queries.push(json!({"did": did, "siid": s, "piid": p}));
            }
        }
    }

    let values_by_key: std::collections::HashMap<String, Value> = if !prop_queries.is_empty() {
        match api.get_devices_prop(&Value::Array(prop_queries)) {
            Ok(results) => results
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            let key = format!("{}:{}:{}", item["did"], item["siid"], item["piid"]);
                            Some((key, item["value"].clone()))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Err(e) => {
                peekoo::log::debug(&format!("get_devices_prop failed: {e}"));
                std::collections::HashMap::new()
            }
        }
    } else {
        std::collections::HashMap::new()
    };

    // Apply filters
    let home_filter = payload["home_id"].as_str().unwrap_or("all");
    let room_filter = payload["room_id"].as_str().unwrap_or("all");

    let mut result_devices = Vec::new();
    for dev in &devices {
        let did = dev["did"].to_string().trim_matches('"').to_string();
        let home_id = dev["home_id"].as_str().unwrap_or("");
        let model = dev["model"].as_str().unwrap_or("");

        let room_info = room_by_did.get(&did).cloned().unwrap_or_else(|| {
            if home_id == "shared" {
                json!({"room_id": "shared", "room_name": "Shared Devices", "home_id": home_id})
            } else {
                json!({"room_id": "unknown", "room_name": "Unassigned", "home_id": home_id})
            }
        });

        if home_filter != "all" && !home_filter.is_empty() && home_id != home_filter {
            continue;
        }
        if room_filter != "all"
            && !room_filter.is_empty()
            && room_info["room_id"].as_str().unwrap_or("") != room_filter
        {
            continue;
        }

        let toggle_prop = model_toggle_map.get(model);
        let quick_toggle = if let Some(tp) = toggle_prop {
            let siid = tp["method"]["siid"].as_i64();
            let piid = tp["method"]["piid"].as_i64();
            if let (Some(s), Some(p)) = (siid, piid) {
                let key = format!("{did}:{s}:{p}");
                json!({
                    "supported": true,
                    "prop_name": tp["name"].as_str().unwrap_or(""),
                    "siid": s,
                    "piid": p,
                    "current": values_by_key.get(&key).cloned().unwrap_or(Value::Null),
                })
            } else {
                json!({"supported": false})
            }
        } else {
            json!({"supported": false})
        };

        let home_name = if home_id == "shared" {
            "Shared".to_string()
        } else {
            home_map
                .get(home_id)
                .and_then(|h| h["name"].as_str())
                .unwrap_or("Shared")
                .to_string()
        };

        result_devices.push(json!({
            "did": did,
            "name": dev["name"].as_str().unwrap_or(did.as_str()),
            "model": model,
            "is_online": dev["isOnline"].as_bool().unwrap_or(false),
            "home_id": home_id,
            "home_name": home_name,
            "room_id": room_info["room_id"],
            "room_name": room_info["room_name"],
            "icon": dev["icon"],
            "quick_toggle": quick_toggle,
            "raw": dev,
        }));
    }

    let homes_payload: Vec<Value> = std::iter::once(json!({"id": "all", "name": "All Homes"}))
        .chain(
            homes
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|h| {
                            json!({
                                "id": h["id"].to_string().trim_matches('"'),
                                "name": h["name"].as_str().unwrap_or("Unnamed Home"),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        )
        .collect();

    ok_response(json!({
        "success": true,
        "authenticated": true,
        "homes": homes_payload,
        "rooms": rooms,
        "devices": result_devices,
    }))
}

fn action_toggle_device(payload: &Value) -> FnResult<String> {
    let api = MijiaApi::load().map_err(|e| Error::msg(e.to_string()))?;
    if !api.is_authenticated() {
        return error_response_code(
            "Please sign in by scanning the QR code first",
            "auth_required",
        );
    }

    let did = payload["did"].as_str().unwrap_or("").trim();
    if did.is_empty() {
        return error_response("did is required");
    }

    let device = find_device(&api, did)?;
    let model = device["model"].as_str().unwrap_or("").to_string();

    let info = device::fetch_device_info(&model).map_err(|e| Error::msg(e.to_string()))?;
    let toggle_prop = find_toggle_property(&info.properties)
        .ok_or_else(|| Error::msg("This device does not support quick toggle"))?;

    let siid = toggle_prop.method.siid;
    let piid = toggle_prop.method.piid;

    // Get current value if target not specified
    let target = if let Some(val) = payload.get("value") {
        val.as_bool().unwrap_or(true)
    } else {
        let curr = api
            .get_devices_prop(&json!({"did": did, "siid": siid, "piid": piid}))
            .map_err(|e| Error::msg(e.to_string()))?;
        !curr["value"].as_bool().unwrap_or(false)
    };

    let ret = api
        .set_devices_prop(&json!({
            "did": did,
            "siid": siid,
            "piid": piid,
            "value": target
        }))
        .map_err(|e| Error::msg(e.to_string()))?;

    ok_response(json!({"success": true, "result": ret, "value": target}))
}

fn action_device_detail(payload: &Value) -> FnResult<String> {
    let api = MijiaApi::load().map_err(|e| Error::msg(e.to_string()))?;
    if !api.is_authenticated() {
        return error_response_code(
            "Please sign in by scanning the QR code first",
            "auth_required",
        );
    }

    let did = payload["did"].as_str().unwrap_or("").trim();
    if did.is_empty() {
        return error_response("did is required");
    }

    let device = find_device(&api, did)?;
    let model = device["model"].as_str().unwrap_or("").to_string();

    let info = device::fetch_device_info(&model).map_err(|e| Error::msg(e.to_string()))?;

    // Query current values for readable properties
    let readable: Vec<Value> = info
        .properties
        .iter()
        .filter(|p| p.rw.contains('r'))
        .map(|p| json!({"did": did, "siid": p.method.siid, "piid": p.method.piid}))
        .collect();

    let current_values: std::collections::HashMap<String, Value> = if !readable.is_empty() {
        match api.get_devices_prop(&Value::Array(readable)) {
            Ok(results) => results
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            let key = format!("{}:{}", item["siid"], item["piid"]);
                            Some((key, item["value"].clone()))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Err(e) => {
                peekoo::log::debug(&format!("get_devices_prop failed: {e}"));
                std::collections::HashMap::new()
            }
        }
    } else {
        std::collections::HashMap::new()
    };

    let normalized_props: Vec<Value> = info
        .properties
        .iter()
        .map(|p| {
            let key = format!("{}:{}", p.method.siid, p.method.piid);
            json!({
                "name": p.name,
                "description": p.description,
                "type": p.prop_type,
                "rw": p.rw,
                "unit": p.unit,
                "range": p.range,
                "value_list": p.value_list,
                "method": {"siid": p.method.siid, "piid": p.method.piid},
                "current_value": current_values.get(&key).cloned().unwrap_or(Value::Null),
            })
        })
        .collect();

    let normalized_actions: Vec<Value> = info
        .actions
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "description": a.description,
                "method": {"siid": a.method.siid, "aiid": a.method.aiid},
            })
        })
        .collect();

    // Energy summary
    let mut current_power = None;
    for prop in &normalized_props {
        let pname = prop["name"].as_str().unwrap_or("").to_lowercase();
        if pname == "power" || pname == "electric-power" {
            current_power = Some(prop["current_value"].clone());
            break;
        }
    }

    let today_usage = compute_today_usage(&normalized_props);
    let month_usage = compute_month_usage(&api, did, &normalized_props);

    ok_response(json!({
        "success": true,
        "device": {
            "did": did,
            "name": device["name"].as_str().unwrap_or(did),
            "model": model,
            "is_online": device["isOnline"].as_bool().unwrap_or(false),
            "raw": device,
        },
        "properties": normalized_props,
        "actions": normalized_actions,
        "energy_summary": {
            "today": today_usage,
            "month": month_usage,
            "current_power": current_power,
        },
    }))
}

fn action_set_property(payload: &Value) -> FnResult<String> {
    let api = MijiaApi::load().map_err(|e| Error::msg(e.to_string()))?;
    if !api.is_authenticated() {
        return error_response_code(
            "Please sign in by scanning the QR code first",
            "auth_required",
        );
    }

    let did = payload["did"].as_str().unwrap_or("").trim();
    let prop_name = payload["prop_name"].as_str().unwrap_or("").trim();
    let value = payload.get("value").cloned().unwrap_or(Value::Null);

    if did.is_empty() || prop_name.is_empty() {
        return error_response("did and prop_name are required");
    }

    let device = find_device(&api, did)?;
    let model = device["model"].as_str().unwrap_or("").to_string();
    let info = device::fetch_device_info(&model).map_err(|e| Error::msg(e.to_string()))?;

    let prop = info
        .properties
        .iter()
        .find(|p| p.name == prop_name || p.name.replace('-', "_") == prop_name)
        .ok_or_else(|| Error::msg(format!("Property '{prop_name}' not found")))?;

    let ret = api
        .set_devices_prop(&json!({
            "did": did,
            "siid": prop.method.siid,
            "piid": prop.method.piid,
            "value": value
        }))
        .map_err(|e| Error::msg(e.to_string()))?;

    ok_response(
        json!({"success": true, "value": ret["value"], "message": "Property updated successfully"}),
    )
}

fn action_run_action(payload: &Value) -> FnResult<String> {
    let api = MijiaApi::load().map_err(|e| Error::msg(e.to_string()))?;
    if !api.is_authenticated() {
        return error_response_code(
            "Please sign in by scanning the QR code first",
            "auth_required",
        );
    }

    let did = payload["did"].as_str().unwrap_or("").trim();
    let action_name = payload["action_name"].as_str().unwrap_or("").trim();
    let value = payload.get("value").cloned();

    if did.is_empty() || action_name.is_empty() {
        return error_response("did and action_name are required");
    }

    let device = find_device(&api, did)?;
    let model = device["model"].as_str().unwrap_or("").to_string();
    let info = device::fetch_device_info(&model).map_err(|e| Error::msg(e.to_string()))?;

    let act = info
        .actions
        .iter()
        .find(|a| a.name == action_name)
        .ok_or_else(|| Error::msg(format!("Action '{action_name}' not found")))?;

    let mut params = json!({
        "did": did,
        "siid": act.method.siid,
        "aiid": act.method.aiid,
    });
    if let Some(v) = value {
        params["value"] = v;
    }

    api.run_action(&params)
        .map_err(|e| Error::msg(e.to_string()))?;

    ok_response(json!({"success": true, "message": "Action executed successfully"}))
}

// ── Helpers ─────────────────────────────────────────────────────────

fn find_device(api: &MijiaApi, did: &str) -> Result<Value, Error> {
    let all = api
        .get_all_devices()
        .map_err(|e| Error::msg(e.to_string()))?;
    let shared = api
        .get_shared_devices_list()
        .map_err(|e| Error::msg(e.to_string()))?;

    let mut all_devices = Vec::new();
    if let Some(arr) = all.as_array() {
        all_devices.extend(arr.clone());
    }
    if let Some(arr) = shared.as_array() {
        all_devices.extend(arr.clone());
    }

    all_devices
        .into_iter()
        .find(|d| d["did"].to_string().trim_matches('"') == did)
        .ok_or_else(|| Error::msg("Device not found"))
}

fn find_toggle_property(properties: &[device::Property]) -> Option<&device::Property> {
    for prop in properties {
        if !prop.rw.contains('w') {
            continue;
        }
        let name = prop.name.to_lowercase();
        if name == "on" {
            return Some(prop);
        }
        if prop.prop_type == "bool" && (name.contains("switch") || name.contains("power")) {
            return Some(prop);
        }
    }
    None
}

fn build_room_index(homes: &Value) -> (std::collections::HashMap<String, Value>, Vec<Value>) {
    let mut room_by_did = std::collections::HashMap::new();
    let mut rooms = vec![json!({"id": "all", "name": "All Rooms", "home_id": "all"})];

    if let Some(home_list) = homes.as_array() {
        for home in home_list {
            let home_id = home["id"].to_string().trim_matches('"').to_string();
            if let Some(room_list) = home["roomlist"].as_array() {
                for room in room_list {
                    let room_id = room["id"].to_string().trim_matches('"').to_string();
                    let room_name = room["name"].as_str().unwrap_or("Unnamed Room");
                    rooms.push(json!({
                        "id": room_id,
                        "name": room_name,
                        "home_id": home_id,
                    }));
                    if let Some(dids) = room["dids"].as_array() {
                        for did in dids {
                            let did_str = did.to_string().trim_matches('"').to_string();
                            room_by_did.insert(
                                did_str,
                                json!({
                                    "room_id": room_id,
                                    "room_name": room_name,
                                    "home_id": home_id,
                                }),
                            );
                        }
                    }
                }
            }
        }
    }

    rooms.push(json!({"id": "shared", "name": "Shared Devices", "home_id": "shared"}));
    (room_by_did, rooms)
}

fn energy_scale_from_prop(prop: &Value) -> f64 {
    let name = prop["name"].as_str().unwrap_or("").to_lowercase();
    let desc = prop["description"].as_str().unwrap_or("").to_lowercase();
    let unit = prop["unit"].as_str().unwrap_or("").to_lowercase();
    let text = format!("{name} {desc} {unit}");
    if text.contains("0.001kwh") {
        0.001
    } else {
        1.0
    }
}

fn compute_today_usage(props: &[Value]) -> Option<f64> {
    for prop in props {
        let pname = prop["name"].as_str().unwrap_or("").to_lowercase();
        if pname.contains("power-consumption")
            || pname.contains("powercost")
            || pname.contains("energy")
        {
            if let Some(curr) = prop["current_value"].as_f64() {
                return Some(curr * energy_scale_from_prop(prop));
            }
        }
    }
    None
}

fn compute_month_usage(api: &MijiaApi, did: &str, props: &[Value]) -> Option<f64> {
    let now = (peekoo::system::time_millis().unwrap_or(0) / 1000) as i64;
    let month_start = now - 40 * 24 * 3600;

    for prop in props {
        let pname = prop["name"].as_str().unwrap_or("").to_lowercase();
        if pname.contains("power-consumption")
            || pname.contains("powercost")
            || pname.contains("energy")
        {
            let method = &prop["method"];
            let siid = method["siid"].as_i64().unwrap_or(0);
            let piid = method["piid"].as_i64().unwrap_or(0);
            let key = format!("{siid}.{piid}");
            let scale = energy_scale_from_prop(prop);

            // Try stat_month_v3 then stat_month
            for data_type in &["stat_month_v3", "stat_month"] {
                let data = json!({
                    "did": did,
                    "key": key,
                    "data_type": data_type,
                    "limit": 1,
                    "time_start": month_start,
                    "time_end": now,
                });
                if let Ok(ret) = api.get_statistics(&data) {
                    if let Some(rows) = ret["result"].as_array() {
                        if let Some(first) = rows.first() {
                            if let Some(val) =
                                parse_stat_value(first["value"].as_str().unwrap_or(""))
                            {
                                return Some(val * scale);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn parse_stat_value(raw: &str) -> Option<f64> {
    if raw.is_empty() {
        return None;
    }
    // Try JSON parse
    if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
        if let Some(arr) = parsed.as_array() {
            if let Some(first) = arr.first() {
                return first.as_f64();
            }
        }
        return parsed.as_f64();
    }
    raw.trim().parse::<f64>().ok()
}
