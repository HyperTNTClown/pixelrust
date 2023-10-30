use std::net::{TcpListener};
use tokio::net::TcpStream;
use std::sync::{Arc, RwLock};
use tokio::io::Interest;
use crate::pixel_map::PixelMap;

mod pixel_map;
mod pixel;
mod render_thread;

fn main() {
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();

	let pixel_map = Arc::new(RwLock::new(PixelMap::new(1280, 720)));

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

async fn handle_connection(socket: std::net::TcpStream, pixel_map: Arc<RwLock<PixelMap>>) {
	let mut buffer = [0; 1024];
	let socket = TcpStream::from_std(socket).unwrap();
	loop {
		if socket.ready(Interest::READABLE).await.is_err() { continue; }
		let pixel_map = pixel_map.clone();
		let bytes_read = socket.try_read(&mut buffer).unwrap_or(0);
		let message = String::from_utf8_lossy(&buffer[..bytes_read]);
		let message = message.trim();
		let mut split = message.split(" ");
		let command = split.next().unwrap();
		match command {
			"PX" => {
				let x = split.next().unwrap().parse::<u32>().unwrap();
				let y = split.next().unwrap().parse::<u32>().unwrap();
				let next = split.next();
				if next.is_none() {
					socket.try_write(format!("PX {} {} {}\n", x, y, pixel_map.read().unwrap().get_pixel(x, y).as_string()).as_bytes()).unwrap();
					continue;
				}
				let hex_color = next.unwrap();
				let r = u8::from_str_radix(&hex_color[0..2], 16).unwrap();
				let g = u8::from_str_radix(&hex_color[2..4], 16).unwrap();
				let b = u8::from_str_radix(&hex_color[4..6], 16).unwrap();
				pixel_map.write().unwrap().set_pixel(x, y, r, g, b);
				socket.try_write(format!("PX {} {} {}\n", x, y, hex_color).as_bytes()).unwrap();
				continue;
			}
			"SIZE" => {
				let size = pixel_map.try_read().unwrap().get_size();
				socket.try_write(format!("SIZE {} {}\n", size.0, size.1).as_bytes()).unwrap();
			}
			"EXIT" => {
				// exit program
				std::process::exit(0);
			}
			_ => {}
		}
	}
}
