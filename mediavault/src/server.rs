use futures::future::{self as f, Future, poll_fn};
use tokio_threadpool::blocking;
use http::{Response, StatusCode};
use warp::{path, Filter, filters};
use hyper::Body;
use serde_json::{json};
use mediavault_common::{
    types as t,
};
use crate::{
    prelude::*,
    app::{self, App},
    storage,
};

fn res_err_json(err: Error) -> http::Response<hyper::Body> {
    let data = serde_json::to_vec(&json!({
        "message": format!("{}", err),
    })).unwrap();

    Response::builder()
        .status(StatusCode::from_u16(500).unwrap())
        .header("content-type", "application/json")
        .body(Body::from(data))
        .unwrap()
}

macro_rules! api_blocking {
    ($app_name:ident : $app_value:expr; | $( $aname:ident : $aty:ty ),* | $e:expr) => {
        move | $( $aname : $aty, )* | {
            let $app_name = $app_value.clone();
            poll_fn(move || blocking(|| {
                let res = $e;
                match res {
                    Ok(data) => {
                        let js = serde_json::to_vec(&data).unwrap();
                        let response = Response::builder()
                            .status(StatusCode::from_u16(200).unwrap())
                            .header("content-type", "application/json")
                            .body(Body::from(js))
                            .unwrap();
                        Ok(response)
                    },
                    Err(e) => Err(e)
                }
            }))
                .then(|res| -> Result<Response<Body>, warp::reject::Rejection> {
                    match res {
                        Ok(Ok(t)) => Ok(t),
                        Ok(Err(e)) => Ok(res_err_json(e)),
                        Err(e) => Ok(res_err_json(Error::from(e))),
                    }
                })
        }
    };
}

pub fn run_server(app: App) {
    // File.
    let a = app.clone();
    let api_file = path!("api" / "file" / String)
        .and(filters::method::get2())
        .and_then(api_blocking!{ app : a.clone(); |hash: String| {
            app.file(&hash)
        }});

    // Files.
    let a = app.clone();
    let api_files = path!("api" / "files")
        .and(filters::method::post2())
        .and(warp::body::json::<t::FileQuery>())
        .and_then(api_blocking!{ app : a.clone(); |q: t::FileQuery| {
            app.files(q.clone())
        }});

    // File update.
    let a = app.clone();
    let api_file_update = path!("api" / "file")
        .and(filters::method::put2())
        .and(warp::body::json::<t::FileUpdate>())
        .and_then(api_blocking!{ app : a.clone(); |data: t::FileUpdate| {
            app.file_update(data.clone())
        }});

    // File delete.
    let a = app.clone();
    let api_file_delete = path!("api" / "file" / String)
        .and(filters::method::delete2())
        .and_then(api_blocking!{ app : a.clone(); |hash: String| {
            app.file_delete(&hash)
                .map(|_| json!({}))
        }});

    let api = api_file
        .or(api_files)
        .or(api_file_update)
        .or(api_file_delete);

    let js_assets = warp::path("assets").and(warp::path("js"))
        .and(warp::fs::dir("../target/web"));

    let index_fallback = warp::any()
        .and(warp::fs::file("../target/web/index.html"));

    let media = warp::path("media")
        .and(warp::fs::dir(app.config.storage_path.clone()));

    let cors = warp::any()
        .and(filters::method::options())
        .map(|| {
            Response::builder()
                .status(StatusCode::from_u16(200).unwrap())
                .header("access-control-allow-origin", "*")
                .header("access-control-allow-methods", "get,post,put")
                .body(Body::empty())
                .unwrap()
        });

    let routes = cors
        .or(api)
        .or(js_assets)
        .or(media)
        .or(index_fallback);

    let routes = routes.with(warp::filters::log::log("mediavault"));

    warp::serve(routes)
        .run(([127, 0, 0, 1], 8080));
}

