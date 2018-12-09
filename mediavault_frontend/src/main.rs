use wasm_bindgen::prelude::*;

use web_sys;

macro_rules! error {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format_args!($($t)*).to_string())))
}

#[macro_use]
mod macros {
    macro_rules! log {
        // Note that this is using the `log` function imported above during
        // `bare_bones`
        ($($t:tt)*) => (web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format_args!($($t)*).to_string())))
    }
}

mod api;
mod views;

#[wasm_bindgen]
pub fn start() {
    log!("starting...");
    let mb = draco::start(
        views::Root::default(),
        draco::select("main").expect("main").into(),
    );

    //    mb.spawn(api::send(), |res| {
    //        match res {
    //            Ok(v) => log!("result: {}", v),
    //            Err(e) => log!("fetch ERROR!"),
    //        }
    //        views::Message::Files(views::files::Message::Test)
    //    });
    mb.send(views::Message::Start);
    mb.send(views::Message::Files(views::files::Message::Query(
        Default::default(),
    )));
}

pub fn main() {}
