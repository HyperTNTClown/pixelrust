use std::sync::atomic::{AtomicU32, AtomicUsize};
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, Rgb};
use crate::pixel::Pixel;

pub(crate) struct PixelMap {
	pixels: Vec<Pixel>,
	width: AtomicU32,
	height: AtomicU32,
	jpeg_cache: Vec<u8>,
	version: AtomicUsize,
	cache_version: AtomicUsize,
}


impl PixelMap {
	pub fn new(width: u32, height: u32) -> PixelMap {
		let mut pixels = Vec::new();
		for x in 0..width {
			for y in 0..height {
				pixels.push(Pixel::new(x, y, 0, 0, 0));
			}
		}
		PixelMap {
			pixels,
			width: AtomicU32::new(width),
			height: AtomicU32::new(height),
			version: AtomicUsize::new(1),
			jpeg_cache: vec![],
			cache_version: AtomicUsize::new(0),
		}
	}

	pub fn get_pixel(&self, x: u32, y: u32) -> Pixel {
		self.pixels[(y + x * self.height.load(Relaxed)) as usize].clone()
	}

	pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
		self.pixels[(y + x * self.height.load(Relaxed)) as usize].set_color(r, g, b);
		self.version.fetch_add(1, SeqCst);
	}

	pub fn update_cache(&mut self, jpeg: Vec<u8>) {
		self.jpeg_cache = jpeg;
		self.cache_version.fetch_add(1, SeqCst);
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


	//TODO: Create a rendering queue and put the edited pixels into it and make the to img_buffer return a Result which can contain a bool or the buffer so that we may use the cached version
	pub fn to_img_buffer(&self) -> (Vec<u8>, bool) {
		if self.cache_version.load(Relaxed) == self.version.load(Relaxed) {
			return (self.jpeg_cache.clone(), true);
		}
		let mut buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
			ImageBuffer::new(
				self.width.load(Relaxed),
				self.height.load(Relaxed),
			);

		for pixel in &self.pixels {
			buffer.put_pixel(
				pixel.x.load(Relaxed),
				pixel.y.load(Relaxed),
				Rgb([
					pixel.r.load(Relaxed),
					pixel.g.load(Relaxed),
					pixel.b.load(Relaxed)
				]),
			);
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
				let pixel = self.pixels[(x * y) as usize].clone();
				pixels.push(pixel);
			}
		}
		PixelMap {
			pixels,
			width: AtomicU32::new(self.width.load(Relaxed)),
			height: AtomicU32::new(self.height.load(Relaxed)),
			version: AtomicUsize::new(self.version.load(Relaxed)),
			jpeg_cache: self.jpeg_cache.clone(),
			cache_version: AtomicUsize::new(self.cache_version.load(Relaxed)),
		}
	}
}

