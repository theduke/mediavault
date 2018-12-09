use futures::Future;
use mediavault_common::types as t;
use std::collections::HashMap;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use web_sys as web;

type Headers = HashMap<String, String>;

pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

pub fn fetch(
    url: &str,
    method: Method,
    body: Option<String>,
    headers: Option<Headers>,
) -> impl Future<Item = String, Error = String> {
    let mut init = web::RequestInit::new();

    let method_literal = match method {
        Method::Get => "get",
        Method::Post => "post",
        Method::Put => "put",
        Method::Delete => "delete",
    };
    init.method(method_literal);
    if let Some(headers) = headers {
        let js_headers = JsValue::from_serde(&headers).unwrap();
        init.headers(&js_headers);
    }
    if let Some(body) = body {
        let js_body = JsValue::from_str(&body);
        init.body(Some(&js_body));
    }

    let request = web::Request::new_with_str_and_init(url, &init).unwrap();
    let promise = web::window().unwrap().fetch_with_request(&request);

    JsFuture::from(promise)
        .map(|response| {
            assert!(response.is_instance_of::<web::Response>());
            response.dyn_into::<web::Response>().unwrap()
        })
        .and_then(|res| res.text())
        .and_then(JsFuture::from)
        .map(|text| text.as_string().unwrap())
        .map_err(|e| format!("{:?}", e))
}

pub fn fetch_json<I, O>(
    url: &str,
    method: Method,
    body: Option<I>,
) -> impl Future<Item = O, Error = String>
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned,
{
    let headers = if body.is_some() {
        let mut h = Headers::new();
        h.insert("content-type".into(), "application/json".into());
        Some(h)
    } else {
        None
    };
    let body = body.map(|b| serde_json::to_string(&b).unwrap());

    fetch(url, method, body, headers)
        .and_then(|raw_body| serde_json::from_str::<O>(&raw_body).map_err(|e| e.to_string()))
}

pub fn file(hash: &str) -> impl Future<Item = t::File, Error = String> {
    fetch_json::<(), _>(&format!("/api/file/{}", hash), Method::Get, None)
}

pub fn files(q: t::FileQuery) -> impl Future<Item = t::FilesPage, Error = String> {
    fetch_json("/api/files", Method::Post, Some(q))
}

pub fn file_update(data: &t::FileUpdate) -> impl Future<Item = t::File, Error = String> {
    // TODO: propagate json encode error?
    fetch_json("/api/file", Method::Put, Some(data.clone()))
}
