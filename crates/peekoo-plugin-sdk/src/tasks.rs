//! Task operations for plugins.
//!
//! Requires the `tasks` permission in `peekoo-plugin.toml`.

use extism_pdk::{Error, Json};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::host_fns::{
    peekoo_task_assign, peekoo_task_create, peekoo_task_delete, peekoo_task_list,
    peekoo_task_toggle, peekoo_task_update, TaskListRequest,
};

fn decode<T: DeserializeOwned>(value: Value, op: &str) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|e| Error::msg(format!("{op} decode error: {e}")))
}

pub fn create<T: DeserializeOwned>(payload: Value) -> Result<T, Error> {
    let response = unsafe { peekoo_task_create(Json(payload))? };
    decode(response.0, "tasks::create")
}

pub fn list<T: DeserializeOwned>(status_filter: Option<&str>) -> Result<Vec<T>, Error> {
    let response = unsafe {
        peekoo_task_list(Json(TaskListRequest {
            status_filter: status_filter.map(ToString::to_string),
        }))?
    };
    decode(response.0, "tasks::list")
}

pub fn update<T: DeserializeOwned>(payload: Value) -> Result<T, Error> {
    let response = unsafe { peekoo_task_update(Json(payload))? };
    decode(response.0, "tasks::update")
}

pub fn delete(id: &str) -> Result<bool, Error> {
    let response = unsafe { peekoo_task_delete(Json(json!({ "id": id })))? };
    Ok(response.0.ok)
}

pub fn toggle<T: DeserializeOwned>(id: &str) -> Result<T, Error> {
    let response = unsafe { peekoo_task_toggle(Json(json!({ "id": id })))? };
    decode(response.0, "tasks::toggle")
}

pub fn assign<T: DeserializeOwned>(id: &str, assignee: &str) -> Result<T, Error> {
    let response = unsafe {
        peekoo_task_assign(Json(json!({
            "id": id,
            "assignee": assignee,
        })))?
    };
    decode(response.0, "tasks::assign")
}
