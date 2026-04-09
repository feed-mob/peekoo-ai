use peekoo_plugin_sdk::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::MijiaError;

const MIOT_SPEC_URL: &str = "https://home.miot-spec.com/spec/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub model: String,
    pub properties: Vec<Property>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub rw: String,
    pub unit: Option<String>,
    pub range: Option<Vec<Value>>,
    #[serde(rename = "value-list")]
    pub value_list: Option<Vec<ValueEntry>>,
    pub method: Method,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueEntry {
    pub value: Value,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub description: String,
    pub method: ActionMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method {
    pub siid: i64,
    pub piid: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMethod {
    pub siid: i64,
    pub aiid: i64,
}

pub fn fetch_device_info(model: &str) -> Result<DeviceInfo, MijiaError> {
    // Check cache first
    if let Some(cached) = load_cached(model) {
        return Ok(cached);
    }

    let url = format!("{MIOT_SPEC_URL}{model}");
    let response = peekoo::http::request(peekoo::http::Request {
        method: "GET",
        url: &url,
        headers: vec![("User-Agent", "Peekoo-Desktop/0.1.0")],
        body: None,
    })
    .map_err(|e| MijiaError::Http {
        status: 0,
        body: e.to_string(),
    })?;

    if response.status >= 400 {
        return Err(MijiaError::Http {
            status: response.status,
            body: response.body,
        });
    }

    let info = parse_spec_page(&response.body, model)?;

    // Cache the result
    save_cached(model, &info);

    Ok(info)
}

fn parse_spec_page(html: &str, model: &str) -> Result<DeviceInfo, MijiaError> {
    let re = Regex::new(r#"data-page="(.*?)">"#)
        .map_err(|e| MijiaError::Parse(format!("regex: {e}")))?;
    let caps = re
        .captures(html)
        .ok_or_else(|| MijiaError::Parse("data-page attribute not found".into()))?;

    let escaped_json = caps.get(1).unwrap().as_str();
    // Unescape HTML entities
    let json_str = escaped_json.replace("&quot;", "\"");
    let page_data: Value = serde_json::from_str(&json_str)
        .map_err(|e| MijiaError::Parse(format!("page JSON: {e}")))?;

    let props = &page_data["props"];

    let (name, resolved_model) = if let Some(product) = props["product"].as_object() {
        let name = product
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(model)
            .to_string();
        let m = product
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(model)
            .to_string();
        (name, m)
    } else {
        let name = props["spec"]["name"].as_str().unwrap_or(model).to_string();
        (name, model.to_string())
    };

    let services = props["spec"]["services"]
        .as_object()
        .ok_or_else(|| MijiaError::Parse("services not found".into()))?;

    let mut properties = Vec::new();
    let mut actions = Vec::new();
    let mut seen_prop_names: Vec<String> = Vec::new();
    let mut seen_action_names: Vec<String> = Vec::new();

    for (siid, service) in services {
        let service_name = service["name"].as_str().unwrap_or_default();

        // Properties
        if let Some(props_map) = service["properties"].as_object() {
            for (piid, prop) in props_map {
                let format = prop["format"].as_str().unwrap_or("string");
                let prop_type = if format.starts_with("int") {
                    "int"
                } else if format.starts_with("uint") {
                    "uint"
                } else {
                    format
                };

                let mut prop_name = prop["name"].as_str().unwrap_or("unknown").to_string();
                if seen_prop_names.contains(&prop_name) {
                    prop_name = format!("{service_name}-{prop_name}");
                }
                seen_prop_names.push(prop_name.clone());

                let desc = format!(
                    "{} / {}",
                    prop["description"].as_str().unwrap_or(""),
                    prop["desc_zh_cn"].as_str().unwrap_or("")
                );

                let mut rw = String::new();
                if let Some(access) = prop["access"].as_array() {
                    if access.iter().any(|a| a.as_str() == Some("read")) {
                        rw.push('r');
                    }
                    if access.iter().any(|a| a.as_str() == Some("write")) {
                        rw.push('w');
                    }
                }

                let value_list = prop["value-list"].as_array().map(|list| {
                    list.iter()
                        .filter_map(|v| {
                            Some(ValueEntry {
                                value: v.get("value")?.clone(),
                                description: v["description"].as_str().unwrap_or("").to_string(),
                            })
                        })
                        .collect()
                });

                let range = prop["value-range"].as_array().map(|r| r.clone());

                properties.push(Property {
                    name: prop_name,
                    description: desc,
                    prop_type: prop_type.to_string(),
                    rw,
                    unit: prop["unit"].as_str().map(|s| s.to_string()),
                    range,
                    value_list,
                    method: Method {
                        siid: siid.parse().unwrap_or(0),
                        piid: piid.parse().unwrap_or(0),
                    },
                });
            }
        }

        // Actions
        if let Some(acts_map) = service["actions"].as_object() {
            for (aiid, act) in acts_map {
                let mut act_name = act["name"].as_str().unwrap_or("unknown").to_string();
                if seen_action_names.contains(&act_name) {
                    act_name = format!("{service_name}-{act_name}");
                }
                seen_action_names.push(act_name.clone());

                let desc = format!(
                    "{} / {}",
                    act["description"].as_str().unwrap_or(""),
                    act["desc_zh_cn"].as_str().unwrap_or("")
                );

                actions.push(Action {
                    name: act_name,
                    description: desc,
                    method: ActionMethod {
                        siid: siid.parse().unwrap_or(0),
                        aiid: aiid.parse().unwrap_or(0),
                    },
                });
            }
        }
    }

    Ok(DeviceInfo {
        name,
        model: resolved_model,
        properties,
        actions,
    })
}

fn cache_state_key(model: &str) -> String {
    format!("mijia-devinfo:{model}")
}

fn load_cached(model: &str) -> Option<DeviceInfo> {
    let raw: String = peekoo::state::get(&cache_state_key(model)).ok()??;
    serde_json::from_str(&raw).ok()
}

fn save_cached(model: &str, info: &DeviceInfo) {
    if let Ok(json) = serde_json::to_string(info) {
        let _ = peekoo::state::set(&cache_state_key(model), &json);
    }
}
