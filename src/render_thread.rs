use std::str;
use std::sync::Arc;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use fastwebsockets::{FragmentCollector, Frame, Payload, Role, WebSocketError};
use sha1::Digest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Handle;

use crate::pixel_map::PixelMap;

pub(crate) async fn render_thread(pixel_map: Arc<PixelMap>, runtime_handle: Handle) {
    let runtime_handle = Arc::new(runtime_handle);
    let arc_handle = Arc::clone(&runtime_handle);

    let server = TcpListener::bind("localhost:1338").await.unwrap();
    loop {
        let (stream, _) = server.accept().await.unwrap();
        let pixel_map = Arc::clone(&pixel_map);
        let arc_handle = Arc::clone(&arc_handle);
        runtime_handle.spawn(
            handle_connection(
                stream,
                Arc::clone(&pixel_map),
                Arc::clone(&arc_handle),
            )
        );
    }
}

// https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers
async fn handle_connection(
    mut stream: TcpStream,
    pixel_map: Arc<PixelMap>,
    runtime_handle: Arc<Handle>,
) -> std::io::Result<()> {
    let mut buffer = [0; 8192];
    let _amount = stream.read(&mut buffer).await?;
    let path = str::from_utf8(&buffer)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();
    if path.contains("canvas") {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: image/qoi\r\n";
        let qoi = pixel_map.to_qoi(runtime_handle.clone()).0;
        stream.write_all(response).await?;
        stream.write_all(b"Dimensions: ").await?;
        stream.write_all(pixel_map.get_width().to_string().as_bytes()).await?;
        stream.write_all(b"x").await?;
        stream.write_all(pixel_map.get_height().to_string().as_bytes()).await?;
        stream.write_all(b"\r\n").await?;
        stream.write_all(b"Content-Length: ").await?;
        stream.write_all(qoi.len().to_string().as_bytes()).await?;
        stream.write_all(b"\r\n\r\n").await?;
        stream.write_all(&qoi).await?;
        stream.flush().await?;
        stream.shutdown().await?;
    } else if path.contains("ws") {
        let response =
            b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n";
        let key = unsafe {
            //println!("{}", str::from_utf8_unchecked(&buffer));
            // IT SHOULD ALREADY BE VALID UTF-8. IF NOT IT WOULD'VE FAILED WHEN PARSING FOR THE PATH
            str::from_utf8_unchecked(&buffer)
                .lines()
                .find(|x| x.to_lowercase().contains("sec-websocket-key"))
                .unwrap()
                .split(": ")
                .nth(1)
                .unwrap()
        };
        let mut sha = sha1::Sha1::new();
        Digest::update(&mut sha, key.as_bytes());
        Digest::update(&mut sha, b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let accept = BASE64_STANDARD.encode(&sha.finalize().0[..]);
        stream.write_all(response).await?;
        stream.write_all(b"Sec-WebSocket-Accept: ").await?;
        stream.write_all(accept.as_bytes()).await?;
        stream.write_all(b"\r\n\r\n").await?;
        let cloned_handle = runtime_handle.clone();
        cloned_handle.spawn(async move {
            let ws = fastwebsockets::WebSocket::after_handshake(stream, Role::Server);
            let mut ws = FragmentCollector::new(ws);
            send_websocket_bytes_deflated(&mut ws, &pixel_map.to_qoi(runtime_handle.clone()).0)
                .await
                .unwrap();
            loop {
                let str = unsafe {
                    match ws.read_frame().await {
                        Ok(e) => String::from_utf8_unchecked(e.payload.to_vec()),
                        Err(e) => {
                            return;
                        }
                    }
                };

                if str.contains("update") {
                    let qoi = pixel_map.to_qoi(runtime_handle.clone());
                    if qoi.1 {
                        // send null-byte to indicate, that nothing has changed
                        send_websocket_bytes_deflated(&mut ws, &*vec![0u8]).await.unwrap();
                    } else {
                        send_websocket_bytes_deflated(&mut ws, &qoi.0).await.unwrap();
                    }
                }
            }
        });
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
