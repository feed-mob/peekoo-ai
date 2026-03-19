use extism_pdk::{Error, Json};

use crate::host_fns::{peekoo_http_request, HttpHeader, HttpRequest, HttpResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: Vec<(&'a str, &'a str)>,
    pub body: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
}

pub fn request(req: Request<'_>) -> Result<Response, Error> {
    let response = unsafe {
        peekoo_http_request(Json(HttpRequest {
            method: req.method.to_string(),
            url: req.url.to_string(),
            headers: req
                .headers
                .into_iter()
                .map(|(name, value)| HttpHeader {
                    name: name.to_string(),
                    value: value.to_string(),
                })
                .collect(),
            body: req.body.map(ToString::to_string),
        }))?
    };

    Ok(Response {
        status: response.0.status,
        body: response.0.body,
        headers: response
            .0
            .headers
            .into_iter()
            .map(|header| (header.name, header.value))
            .collect(),
    })
}

#[allow(dead_code)]
fn _assert_http_response(_: HttpResponse) {}
