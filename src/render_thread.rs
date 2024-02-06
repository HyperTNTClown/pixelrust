use crate::pixel_map::PixelMap;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use fastwebsockets::{FragmentCollector, Frame, Payload, Role, WebSocketError};
use sha1::Digest;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::{str, thread};
use tokio::net::TcpStream;
use tokio::runtime::Handle;

pub(crate) fn render_thread(pixel_map: Arc<PixelMap>, runtime_handle: Handle) {
    let arc_handle = Arc::new(runtime_handle);

    let server = TcpListener::bind("localhost:1338").unwrap();
    for stream in server.incoming() {
        let pixel_map = Arc::clone(&pixel_map);
        let arc_handle = Arc::clone(&arc_handle);
        thread::spawn(move || {
            handle_connection(
                stream.unwrap(),
                Arc::clone(&pixel_map),
                Arc::clone(&arc_handle),
            )
            .unwrap();
        });
    }
}

// https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers
fn handle_connection(
    mut stream: std::net::TcpStream,
    pixel_map: Arc<PixelMap>,
    runtime_handle: Arc<Handle>,
) -> std::io::Result<()> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer)?;
    let path = std::str::from_utf8(&buffer)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .skip(1)
        .next()
        .unwrap();
    if path.contains("canvas") {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: image/qoi\r\n";
        let qoi = pixel_map.to_qoi();
        stream.write(response)?;
        stream.write(b"Content-Length: ")?;
        stream.write(qoi.len().to_string().as_bytes())?;
        stream.write(b"\r\n\r\n")?;
        stream.write(&**qoi)?;
        stream.flush()?;
        stream.shutdown(std::net::Shutdown::Both)?;
    } else if path.contains("ws") {
        let response =
            b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n";
        let key = unsafe {
            // IT SHOULD ALREADY BE VALID UTF-8. IF NOT IT WOULD'VE FAILED WHEN PARSING FOR THE PATH
            std::str::from_utf8_unchecked(&buffer)
                .lines()
                .find(|x| x.to_lowercase().contains("sec-websocket-key: "))
                .unwrap()
                .split(": ")
                .skip(1)
                .next()
                .unwrap()
        };
        let mut sha = sha1::Sha1::new();
        Digest::update(&mut sha, key.as_bytes());
        Digest::update(&mut sha, b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let accept = BASE64_STANDARD.encode(&sha.finalize().0[..]);
        stream.write(response)?;
        stream.write(b"Sec-WebSocket-Accept: ")?;
        stream.write(accept.as_bytes())?;
        stream.write(b"\r\n\r\n")?;
        runtime_handle.block_on(async move {
            tokio::spawn(async move {
                let tokio_stream = TcpStream::from_std(stream).unwrap();
                let ws = fastwebsockets::WebSocket::after_handshake(tokio_stream, Role::Server);
                let mut ws = FragmentCollector::new(ws);
                send_websocket_bytes_deflated(&mut ws, &*pixel_map.to_qoi())
                    .await
                    .unwrap();
                loop {
                    let str = unsafe {
                        match ws.read_frame().await {
                            Ok(e) => String::from_utf8_unchecked(e.payload.to_vec()),
                            Err(_) => "".to_string(),
                        }
                    };

                    if str.contains("update") {
                        let qoi = pixel_map.to_qoi();
                        send_websocket_bytes_deflated(&mut ws, &*qoi).await.unwrap();
                    }
                }
            })
        });
    } else {
        // TODO: SWITCH TO WEBSOCKETS SO THE CLIENT CAN SIMPLY REQUEST THE NEW IMAGE (TO NOT OVERWHELM THEM) AND IT DOES NOT NEED TO BE IN BASE64 (SIZE SAVINGS)
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n";
        stream.write(response)?;
        stream.write(b"Cache-Control: no-cache\r\n")?;
        stream.write(b"Connection: keep-alive\r\n")?;
        stream.write(b"Content-Encoding: br\r\n")?;
        stream.write(b"\r\n")?;
        stream.flush()?;
        let mut comp_writer = brotli::CompressorWriter::new(&mut stream, 4096, 11, 22);
        loop {
            // A bit of delay
            //thread::sleep(Duration::from_millis(16));
            let qoi = pixel_map.to_qoi();
            let base = BASE64_STANDARD.encode(&*qoi);
            comp_writer.write(b"data: ")?;
            comp_writer.write_all(&*base.as_bytes())?;
            comp_writer.write(b"\n\n")?;
            comp_writer.flush()?;
        }
        //loop {
        //    match arc.recv_timeout(Duration::from_secs(1)) {
        //        Ok(px) => {
        //            let v = arc.try_iter().map(|x| x.to_utf8()).collect::<Vec<String>>();
        //            let e = px.to_utf8() + ";" + &*v.join(";");
        //            comp_writer.write(b"data: ").unwrap();
        //            comp_writer.write_all(&*e.as_bytes()).unwrap();
        //            comp_writer.write(b"\n\n")?;
        //            comp_writer.flush()?;
        //        }
        //        Err(_) => {
        //            comp_writer.write(b":keepalive\n\n")?;
        //            comp_writer.flush()?;
        //        }
        //    }
        //}
    }
    Ok(())
}

async fn send_websocket_bytes_deflated(
    ws: &mut FragmentCollector<TcpStream>,
    data: &[u8],
) -> Result<(), WebSocketError> {
    let comp = fdeflate::compress_to_vec(data);
    ws.write_frame(Frame::binary(Payload::Owned(comp))).await
}

#[allow(dead_code)]
async fn send_websocket_str_deflated(
    ws: &mut FragmentCollector<TcpStream>,
    data: &str,
) -> Result<(), WebSocketError> {
    println!("Sending: {}", data);
    send_websocket_bytes_deflated(ws, data.as_bytes()).await
}
