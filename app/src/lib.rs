use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::ArrayBuffer;
use web_sys::{
    Blob, CanvasRenderingContext2d, EventSource, HtmlCanvasElement, ImageData, WebSocket,
};

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
    let ctx: CanvasRenderingContext2d = el.get_context("2d").unwrap().unwrap().dyn_into().unwrap();
    let e = JsFuture::from(web_sys::window().unwrap().fetch_with_str("/api/canvas"))
        .await
        .unwrap();
    let res = e.unchecked_into::<web_sys::Response>();
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
            1280,
            720,
        )
        .unwrap();
        ctx.put_image_data(&img, 0.0, 0.0).unwrap();
    }

    let ws = WebSocket::new("wss://localhost/api/ws").unwrap();

    //let ev = EventSource::new("/api/pixels").unwrap();
    let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
        //web_sys::console::log_1(&e);
        let e = e.unchecked_into::<web_sys::MessageEvent>();
        web_sys::console::log_1(&e);
        // DATA IS A BASE64 QOI
        //let data = e.data().as_string().unwrap();
        //let mut bytes = BASE64_STANDARD.decode(data).unwrap();
        //let t = web_sys::window().unwrap().performance().unwrap().now();
        //let t1 = web_sys::window().unwrap().performance().unwrap().now();
        //web_sys::console::log_1(&format!("Decoding: {}", t1 - t).into());
        //let img = ImageData::new_with_u8_clamped_array_and_sh(
        //    wasm_bindgen::Clamped(&*rapid_qoi::Qoi::decode_alloc(&*bytes).unwrap().1),
        //    1280,
        //    720,
        //)
        //.unwrap();
        //ctx.put_image_data(&img, 0.0, 0.0).unwrap();
        //let t2 = web_sys::window().unwrap().performance().unwrap().now();
        //web_sys::console::log_1(&format!("Rendering: {}", t2 - t1).into());
        //let mut split = data.split(";");
        //split.map(|x| x.split("_")).for_each(|mut split_split| {
        //    let x = match split_split.next() {
        //        Some(x) => x.parse::<u32>().unwrap(),
        //        None => return,
        //    };
        //    let y = split_split.next().unwrap().parse::<u32>().unwrap();
        //    let color = split_split.next().unwrap();
        //    ctx.set_fill_style(&JsValue::from_str(&*("#".to_owned() + color)));
        //    ctx.fill_rect(x as f64, y as f64, 1.0, 1.0);
        //})
    }) as Box<dyn FnMut(_)>);

    //ev.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())
    //    .unwrap();

    ws.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())
        .unwrap();

    closure.forget();
}
