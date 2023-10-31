use std::sync::atomic::{AtomicU32, AtomicUsize};
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use image::codecs::jpeg::JpegEncoder;
use image::{ColorType, ImageBuffer, Pixel, Rgb};
use crate::color::Color;

pub(crate) struct PixelMap {
	pixels: Vec<AtomicU32>,
	width: AtomicU32,
	height: AtomicU32,
	version: AtomicUsize,
}


impl PixelMap {
	pub fn new(width: u32, height: u32) -> PixelMap {
		let mut pixels = Vec::new();
		for x in 0..width {
			for y in 0..height {
				pixels.push(AtomicU32::new(Color::black().raw()));
			}
		}
		PixelMap {
			pixels,
			width: AtomicU32::new(width),
			height: AtomicU32::new(height),
			version: AtomicUsize::new(1),
		}
	}

	pub fn get_color(&self, x: u32, y: u32) -> Color {
		Color::new(self.pixels[(x + y * self.width.load(Relaxed)) as usize].load(Relaxed))
	}

	pub fn get_pixel(&self, x: u32, y: u32) -> &AtomicU32 {
		&self.version.fetch_add(1, SeqCst);
		&self.pixels[(x + y * self.width.load(Relaxed)) as usize]
	}

	pub fn get_width(&self) -> u32 {
		self.width.load(Relaxed)
	}

	pub fn get_height(&self) -> u32 {
		self.height.load(Relaxed)
	}

	pub fn get_size(&self) -> (u32, u32) {
		(self.get_width(), self.get_height())
	}

	pub fn to_img_buffer(&self) -> (Vec<u8>, bool) {
		if self.version.load(Relaxed) == 0 {
			return (vec![0u8], true);
		}
		println!("Updating image buffer");
		&self.version.store(0, SeqCst);
		let mut buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
			ImageBuffer::new(
				self.width.load(Relaxed),
				self.height.load(Relaxed),
			);


		let len = self.pixels.len();
		for i in 0..len {
			let pixel = &self.pixels[i];

			let x = i as u32 % self.width.load(Relaxed);
			let y = i as u32 / self.width.load(Relaxed);
			let color = Color::new(pixel.load(Relaxed));

			buffer.put_pixel(x, y, Rgb([color.r(), color.g(), color.b()]));
		}

		let mut jpeg_buffer = Vec::new();
		let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buffer, 60);
		encoder.encode(
			&buffer.into_raw(),
			self.width.load(Relaxed),
			self.height.load(Relaxed),
			image::ColorType::Rgb8)
			.unwrap();

		(jpeg_buffer, false)
	}
}

impl Clone for PixelMap {
	fn clone(&self) -> Self {
		let mut pixels = Vec::new();
		for x in 0..self.width.load(Relaxed) {
			for y in 0..self.height.load(Relaxed) {
				let pixel = self.pixels[(x * y) as usize].load(Relaxed);
				pixels.push(AtomicU32::new(pixel));
			}
		}
		PixelMap {
			pixels,
			width: AtomicU32::new(self.width.load(Relaxed)),
			height: AtomicU32::new(self.height.load(Relaxed)),
			version: AtomicUsize::new(self.version.load(Relaxed)),
		}
	}
}

