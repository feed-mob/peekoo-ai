use extism_pdk::{Error, Json};

use crate::host_fns::{
    peekoo_secret_delete, peekoo_secret_get, peekoo_secret_set, SecretDeleteRequest,
    SecretGetRequest, SecretSetRequest,
};

pub fn get(key: &str) -> Result<Option<String>, Error> {
    let response = unsafe {
        peekoo_secret_get(Json(SecretGetRequest {
            key: key.to_string(),
        }))?
    };

    Ok(response.0.value)
}

pub fn set(key: &str, value: &str) -> Result<(), Error> {
    unsafe {
        peekoo_secret_set(Json(SecretSetRequest {
            key: key.to_string(),
            value: value.to_string(),
        }))?;
    }

    Ok(())
}

pub fn delete(key: &str) -> Result<(), Error> {
    unsafe {
        peekoo_secret_delete(Json(SecretDeleteRequest {
            key: key.to_string(),
        }))?;
    }

    Ok(())
}
