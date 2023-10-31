use std::sync::Arc;
use std::thread;
use crate::pixel_map::PixelMap;
use mjpeg_rs::MJpeg;

pub(crate) fn render_thread(pixel_map: Arc<PixelMap>) {

	let m = Arc::new(MJpeg::new());
	let mrc = m.clone();

	let pixel_map = Arc::clone(&pixel_map);

	thread::spawn(move || {
		let mut cache: Vec<u8>;
		let (buf, _) = pixel_map.to_img_buffer();
		m.update_jpeg(buf.clone()).unwrap();
		cache = buf.clone();
		loop {
			let (buf, valid) = pixel_map.to_img_buffer();
			if valid {
				m.update_jpeg(cache.clone()).unwrap();
				continue;
			}
			cache = buf.clone();
			m.update_jpeg(buf).unwrap();
			thread::sleep(std::time::Duration::from_millis(100));
		}
	});

	mrc.run("0.0.0.0:1338").unwrap();
}
