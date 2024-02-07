use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::ArrayBuffer;
use web_sys::{Blob, CanvasRenderingContext2d, HtmlCanvasElement, ImageData, WebSocket};

#[wasm_bindgen(start)]
async fn main() {
    console_error_panic_hook::set_once();

    web_sys::console::log_1(&"Hello from Rust!".into());

    let binding = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("canvas")
        .unwrap();

    let el: &HtmlCanvasElement = binding.dyn_ref::<HtmlCanvasElement>().unwrap();
    let mut ctx: CanvasRenderingContext2d =
        el.get_context("2d").unwrap().unwrap().dyn_into().unwrap();
    let e = JsFuture::from(web_sys::window().unwrap().fetch_with_str("/api/canvas"))
        .await
        .unwrap();
    let res = e.unchecked_into::<web_sys::Response>();
    let dimensions = res
        .headers()
        .get("Dimensions")
        .unwrap()
        .unwrap()
        .split("x")
        .map(|s| s.parse::<u32>().unwrap())
        .collect::<Vec<u32>>();
    let s = JsFuture::from(res.blob().unwrap()).await.unwrap();
    let blob = Blob::from(s);
    let arr: ArrayBuffer = JsFuture::from(blob.array_buffer())
        .await
        .unwrap()
        .unchecked_into();
    let vec = js_sys::Uint8Array::new(&arr).to_vec();
    {
        // Scoped to drop large image data quickly
        let img = ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&*rapid_qoi::Qoi::decode_alloc(&*vec).unwrap().1),
            dimensions[0],
            dimensions[1],
        )
        .unwrap();
        ctx.put_image_data(&img, 0.0, 0.0).unwrap();
    }

    let ws = match web_sys::window()
        .unwrap()
        .location()
        .protocol()
        .unwrap()
        .as_str()
    {
        "http:" => WebSocket::new(
            &("ws://".to_owned()
                + &*web_sys::window().unwrap().location().host().unwrap()
                + "/api/ws"),
        ),
        "https:" | _ => WebSocket::new(
            &("wss://".to_owned()
                + &*web_sys::window().unwrap().location().host().unwrap()
                + "/api/ws"),
        ),
    }
    .unwrap();
    let mut closure_ws = ws.clone();

    //let ev = EventSource::new("/api/pixels").unwrap();
    let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
        let dimensions = &dimensions;
        let dimensions = dimensions.clone();
        let closure_ws = &mut closure_ws;
        let mut cl_ws = closure_ws.clone();
        let e = e.unchecked_into::<web_sys::MessageEvent>();
        let blob = e.data().unchecked_into::<Blob>();
        let ctx = &mut ctx;
        let mut ctx = ctx.clone();
        let cl = Closure::wrap(Box::new(move |arr| {
            let dimensions = &dimensions;
            let cl_ws = &mut cl_ws;
            let vec = js_sys::Uint8Array::new(&arr).to_vec();
            let data = fdeflate::decompress_to_vec(&vec).unwrap();
            let img = ImageData::new_with_u8_clamped_array_and_sh(
                wasm_bindgen::Clamped(&*rapid_qoi::Qoi::decode_alloc(&*data).unwrap().1),
                dimensions[0],
                dimensions[1],
            );
            let ctx = &mut ctx;
            let ctx = ctx.clone();
            ctx.put_image_data(&img.unwrap(), 0.0, 0.0).unwrap();
            cl_ws.send_with_str("update").unwrap();
        }) as Box<dyn FnMut(_)>);
        let _ = blob.array_buffer().then(&cl);
        std::mem::forget(cl);
    }) as Box<dyn FnMut(_)>);

    ws.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())
        .unwrap();

    closure.forget();
}
