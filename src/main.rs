use crate::color::Color;
use crate::pixel_map::PixelMap;
use crossbeam_channel::internal::SelectHandle;
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::str::FromStr;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

mod color;
mod pixel_map;
mod render_thread;

static WIDTH: u32 = 1280;
static HEIGHT: u32 = 720;

fn main() {
    let (tx, rx) = crossbeam_channel::unbounded::<PX>();
    let tx_arc = Arc::new(tx);
    let rx_arc = Arc::new(rx);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let handle = runtime.handle().clone();

    let pixel_map = Arc::new(PixelMap::load_image("image.qoi"));

    let pix_clone = Arc::clone(&pixel_map);

    std::thread::spawn(move || {
        runtime.block_on(async move {
            let tcp_listener = TcpListener::bind("0.0.0.0:1337").unwrap();
            loop {
                let tx = Arc::clone(&tx_arc);
                let (socket, _) = tcp_listener.accept().unwrap();
                let pixel_map = Arc::clone(&pixel_map);
                tokio::spawn(async move {
                    let socket = TcpStream::from_std(socket).unwrap();
                    handle_connection(socket, pixel_map, Arc::clone(&tx)).await;
                });
            }
        })
    });

    // TODO: Array as background worker (to not block) on frontend - store in array (maybe localstorage???) because received data from server might not be complete.
    //  - Send Data in raw binary tcp sockets, coordinates in one u32, color in another u32
    //  - Maybe, just maybe add a timestamp to the data to prevent old data from overwriting new data
    //  - Serve a webp or avif image as starting point for the frontend to easily load the image to the canvas
    //  - Think about batching and compression for sending stuff to the frontend. We wouldn't want to have too much egress traffic

    render_thread::render_thread(pix_clone, rx_arc, handle);
}

#[derive(Debug)]
struct PX {
    x: u32,
    y: u32,
    color: Color,
}

impl PX {
    fn to_utf8(&self) -> String {
        format!("{}_{}_{}", self.x, self.y, self.color.hex())
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    pixel_map: Arc<PixelMap>,
    tx: Arc<crossbeam_channel::Sender<PX>>,
) {
    let mut debug = false;
    let (read_half, mut write_half) = socket.split();
    let mut message = String::new();
    let mut reader = BufReader::new(read_half);
    loop {
        let pixel_map = pixel_map.clone();
        message.clear();
        match reader.read_line(&mut message).await {
            Ok(0) => break,
            Ok(_) => {
                let message = message.trim();
                let mut split = message.split(" ");
                let command = split.next().unwrap();
                match command {
                    "PX" => {
                        let x: u32 = match split.next() {
                            Some(x) => x.parse::<u32>().unwrap(),
                            None => {
                                write_half
                                    .write("ERR: Missing X\n".as_bytes())
                                    .await
                                    .unwrap();
                                continue;
                            }
                        };
                        let y: u32 = match split.next() {
                            Some(y) => y.parse::<u32>().unwrap(),
                            None => {
                                write_half
                                    .write("ERR: Missing Y\n".as_bytes())
                                    .await
                                    .unwrap();
                                continue;
                            }
                        };
                        match (x, y) {
                            coords if coords.0 == WIDTH || coords.1 == HEIGHT => {
                                write_half
                                    .write("ERR: 0 based index...\n".as_bytes())
                                    .await
                                    .unwrap();
                                continue;
                            }
                            coords if coords.0 > WIDTH || coords.1 > HEIGHT => {
                                write_half
                                    .write("ERR: Out of Bounds (Tip: SIZE)\n".as_bytes())
                                    .await
                                    .unwrap();
                                continue;
                            }
                            _ => {}
                        };
                        let next = split.next();
                        if next.is_none() {
                            write_half
                                .write(
                                    format!("PX {} {} {}\n", x, y, pixel_map.get_color(x, y))
                                        .as_bytes(),
                                )
                                .await
                                .unwrap();
                            continue;
                        }
                        let hex_color = next.unwrap();
                        let color = Color::from_hex(hex_color).unwrap();
                        let mut original_color = pixel_map.get_color(x, y);
                        original_color.overlay_mut(color);
                        if debug {
                            write_half
                                .write(format!("PX {} {} {}\n", x, y, hex_color).as_bytes())
                                .await
                                .unwrap_or(0);
                            println!("PX {} {} {}", x, y, hex_color);
                        }
                        if original_color.equals(pixel_map.get_color(x, y)) {
                            continue;
                        }
                        pixel_map
                            .get_pixel(x, y)
                            .store(original_color.raw(), Relaxed);
                        tx.try_send(PX {
                            x,
                            y,
                            color: original_color,
                        })
                        .unwrap();
                    }
                    "SIZE" => {
                        let size = pixel_map.get_size();
                        write_half
                            .write(format!("SIZE {} {}\n", size.0, size.1).as_bytes())
                            .await
                            .unwrap();
                    }
                    "EXIT" => {
                        // exit program
                        write_half.write("EXITING\n".as_bytes()).await.unwrap();
                        write_half.flush().await.unwrap();
                        return;
                    }
                    "DEBUG" => {
                        debug = !debug;
                    }
                    "HELP" => {
                        write_half
                            .write("Commands:\nPX x y [hex]\nSIZE\nEXIT\nDEBUG\nHELP\n".as_bytes())
                            .await
                            .unwrap();
                    }
                    _ => {
                        write_half
                            .write("ERR: Unknown Command\n".as_bytes())
                            .await
                            .unwrap();
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
        write_half.flush().await.unwrap();
    }
}
