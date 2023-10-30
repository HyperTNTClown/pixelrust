use std::sync::{Arc, RwLock};
use crate::pixel_map::PixelMap;
use mjpeg_rs::MJpeg;

pub(crate) fn render_thread(pixel_map: Arc<RwLock<PixelMap>>) {

	let m = Arc::new(MJpeg::new());
	let mrc = m.clone();

	let _pixel_map = Arc::clone(&pixel_map);

	std::thread::spawn(move || {
		loop {
			let (buf, valid) = match _pixel_map.try_read() {
				Ok(p) => {
					let img_buffer = p.to_img_buffer();
					m.update_jpeg(img_buffer.0.clone()).unwrap();
					img_buffer
				},
				Err(_) => continue
			};

			if valid {
				continue;
			}

			match _pixel_map.try_write() {
				Ok(mut p) => {
					p.update_cache(buf);
				},
				Err(_) => continue
			}
		}
	});

	mrc.run("0.0.0.0:1338").unwrap();
}
