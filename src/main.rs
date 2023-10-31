use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener};
use tokio::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;
use tokio::io::{AsyncRead, AsyncWrite, Interest, ReadBuf};
use crate::color::Color;
use crate::pixel_map::PixelMap;

mod pixel_map;
mod render_thread;
mod color;

static WIDTH: u32 = 1280;
static HEIGHT: u32 = 720;

fn main() {
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();

	let pixel_map = Arc::new(PixelMap::new(WIDTH, HEIGHT));

	let pix_clone = Arc::clone(&pixel_map);

	std::thread::spawn(move || runtime.block_on(async move {
		let tcp_listener = TcpListener::bind("0.0.0.0:1337").unwrap();
		loop {
			let (socket, _) = tcp_listener.accept().unwrap();
			let pixel_map = Arc::clone(&pixel_map);
			tokio::spawn(async {
				handle_connection(socket, pixel_map).await;
			});
		}
	}));

	render_thread::render_thread(pix_clone);
}

async fn handle_connection(mut socket: std::net::TcpStream, pixel_map: Arc<PixelMap>) {
	let mut message = String::new();
	socket.set_nonblocking(true).unwrap();
	socket.set_nodelay(true).unwrap();
	let mut reader = BufReader::new(socket.try_clone().unwrap());
	println!("Connection from {}", socket.peer_addr().unwrap());
	loop {
		let pixel_map = pixel_map.clone();
		message.clear();
		let bytes_read = reader.read_line(&mut message);
		let message = message.trim();
		let mut split = message.split(" ");
		let command = split.next().unwrap();
		println!("Message: {}, Command: {}", message, command);
		match command {
			"PX" => {
				let x = split.next().unwrap().parse::<u32>().unwrap();
				let y = split.next().unwrap().parse::<u32>().unwrap();
				println!("X: {}, Y: {}", x, y);
				match (x,y) {
					coords
					if coords.0 == WIDTH || coords.1 == HEIGHT => {
						socket.write("ERR: 0 based index...\n".as_bytes()).unwrap();
						continue;
					}
					coords
					if coords.0 > WIDTH || coords.1 > HEIGHT => {
						socket.write("ERR: Out of Bounds (Tip: SIZE)\n".as_bytes()).unwrap();
						continue;
					}
					_ => {}
				};
				let next = split.next();
				if next.is_none() {
					socket.write(format!("PX {} {} {}\n", x, y, pixel_map.get_color(x, y)).as_bytes()).unwrap();
					continue;
				}
				let hex_color = next.unwrap();
				let r = u8::from_str_radix(&hex_color[0..2], 16).unwrap();
				let g = u8::from_str_radix(&hex_color[2..4], 16).unwrap();
				let b = u8::from_str_radix(&hex_color[4..6], 16).unwrap();
				println!("R: {}, G: {}, B: {}", r, g, b);

				pixel_map.get_pixel(x, y).store(Color::from_rgb(r, g, b).raw(), Relaxed);
				socket.write(format!("PX {} {} {}\n", x, y, hex_color).as_bytes()).unwrap();
				println!("PX {} {} {}", x, y, hex_color);
			},
			"SIZE" => {
				let size = pixel_map.get_size();
				socket.write(format!("SIZE {} {}\n", size.0, size.1).as_bytes()).unwrap();
			},
			"EXIT" => {
				// exit program
				socket.write("EXITING\n".as_bytes()).unwrap();
				std::process::exit(0);
			}
			_ => {
				socket.write("ERR: Unknown Command\n".as_bytes()).unwrap();
			}
		}
	}
}
