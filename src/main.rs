use crate::color::Color;
use crate::pixel_map::PixelMap;
use std::net::TcpListener;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

mod color;
mod pixel_map;
mod render_thread;

static WIDTH: u32 = 1280;
static HEIGHT: u32 = 720;

fn main() {
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
                let (socket, _) = tcp_listener.accept().unwrap();
                let pixel_map = Arc::clone(&pixel_map);
                tokio::spawn(async move {
                    let socket = TcpStream::from_std(socket).unwrap();
                    handle_connection(socket, pixel_map).await;
                });
            }
        })
    });

    render_thread::render_thread(pix_clone, handle);
}

async fn handle_connection(mut socket: TcpStream, mut pixel_map: Arc<PixelMap>) {
    let mut binary = false;
    let mut debug = false;
    let (read_half, mut write_half) = socket.split();
    let mut message = String::new();
    /* Binary Message Buffer
    // Format:
    // [u16: x][u16: y][u32: rgba]
    //
    // 2 bytes for x, 2 bytes for y, 4 bytes for rgb = 8 bytes (better padding than 7, so no rgb)
    //
    // 8 bytes * 1280 * 720 = 7_372_800 bytes = 7.3728 MB
    //
    // Instead of:
    // - 2 bytes for 'PX',
    // - 1 byte for ' ',
    // - ~3 bytes for x,
    // - 1 byte for ' ',
    // - ~3 bytes for y,
    // - 1 byte for ' ',
    // - 6 bytes for hex,
    // - 1 byte for '\n'
    // = 18 bytes * 1280 * 720 = 18_432_000 bytes = 18.432 MB
    //
    // 55.56% less data
     */
    let mut bin_buf: [u8; 8] = [0; 8];
    let mut reader = BufReader::new(read_half);
    loop {
        let pixel_map = &mut pixel_map;
        message.clear();
        if binary {
            let bin_read_length = reader.read_exact(&mut bin_buf).await;
            match bin_read_length {
                Ok(e) => {
                    if e != 8 {
                        write_half
                            .write("ERR: Invalid Binary Length\n".as_bytes())
                            .await
                            .unwrap();
                        continue;
                    }
                    let x = u16::from_le_bytes([bin_buf[0], bin_buf[1]]) as u32;
                    let y = u16::from_le_bytes([bin_buf[2], bin_buf[3]]) as u32;
                    if x >= WIDTH || y >= HEIGHT {
                        write_half
                            .write("ERR: Out of Bounds (Tip: SIZE)\n".as_bytes())
                            .await
                            .unwrap();
                        continue;
                    }
                    let color = Color::new(u32::from_le_bytes([
                        bin_buf[4], bin_buf[5], bin_buf[6], bin_buf[7],
                    ]));
                    let mut original_color = pixel_map.get_color(x, y);
                    original_color.overlay_mut(color);
                    if debug {
                        write_half
                            .write(format!("PX {} {} {}\n", x, y, color.hex()).as_bytes())
                            .await
                            .unwrap_or(0);
                        println!("PX {} {} {}", x, y, color.hex());
                    }
                    if original_color.equals(pixel_map.get_color(x, y)) {
                        continue;
                    }
                    pixel_map
                        .get_pixel(x, y)
                        .store(original_color.raw(), Relaxed);
                }
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
            }
            continue;
        }
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
                    "BIN" => {
                        binary = !binary;
                        write_half.write(b"\xac\xce\x91").await.unwrap();
                    }
                    "HELP" => {
                        write_half
                            .write("Commands:\nPX x y [hex]\nSIZE\nEXIT\nDEBUG\nBIN (changes channel mode: [x:u16][y:u16][rgba:u32] LE)\nHELP\n".as_bytes())
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
