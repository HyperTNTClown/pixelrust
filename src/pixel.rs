use std::sync::atomic::{AtomicU32, AtomicU8};

pub struct Pixel {
	pub(crate) r: AtomicU8,
	pub(crate) g: AtomicU8,
	pub(crate) b: AtomicU8,
	pub(crate) x:AtomicU32,
	pub(crate) y: AtomicU32
}

impl Pixel {
	pub fn new(x: u32, y: u32, r: u8, g: u8, b: u8) -> Pixel {
		Pixel {
			x: AtomicU32::new(x),
			y: AtomicU32::new(y),
			r: AtomicU8::new(r),
			g: AtomicU8::new(g),
			b: AtomicU8::new(b),
		}
	}

	pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
		self.r.store(r, std::sync::atomic::Ordering::Relaxed);
		self.g.store(g, std::sync::atomic::Ordering::Relaxed);
		self.b.store(b, std::sync::atomic::Ordering::Relaxed);
	}

	pub fn as_string(&self) -> String {
		let tmp = format!("{} {} {}", self.r.load(std::sync::atomic::Ordering::Relaxed), self.g.load(std::sync::atomic::Ordering::Relaxed), self.b.load(std::sync::atomic::Ordering::Relaxed));
		tmp
	}
}

impl Clone for Pixel {
	fn clone(&self) -> Self {
		Pixel {
			x: AtomicU32::new(self.x.load(std::sync::atomic::Ordering::Relaxed)),
			y: AtomicU32::new(self.y.load(std::sync::atomic::Ordering::Relaxed)),
			r: AtomicU8::new(self.r.load(std::sync::atomic::Ordering::Relaxed)),
			g: AtomicU8::new(self.g.load(std::sync::atomic::Ordering::Relaxed)),
			b: AtomicU8::new(self.b.load(std::sync::atomic::Ordering::Relaxed)),
		}
	}
}
