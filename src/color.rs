use std::fmt::Display;
use std::num::ParseIntError;

pub struct Color {
	value: u32,
}

impl Color {
	pub const fn new(value: u32) -> Color {
		Color { value }
	}

	pub const fn from_rgb(r: u8, g: u8, b: u8) -> Color {
		Color::from_rgba(r, g, b, 255)
	}

	pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
		Color {
			value: (r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | (a as u32)
		}
	}

	pub fn r(&self) -> u8 {
		((self.value >> 24) & 0xFF) as u8
	}

	pub fn g(&self) -> u8 {
		((self.value >> 16) & 0xFF) as u8
	}

	pub fn b(&self) -> u8 {
		((self.value >> 8) & 0xFF) as u8
	}

	pub fn a(&self) -> u8 {
		(self.value & 0xFF) as u8
	}

	pub fn from_hex(hex: &str) -> Result<Self, ParseIntError> {
		let mut raw = u32::from_str_radix(hex, 16)?;

		if hex.len() == 6 {
			raw = raw << 8 | 0xFF;
		}

		Ok(Color::new(raw))
	}

	pub fn hex(&self) -> String {
		format!("{:06x}", self.value >> 8)
	}

	pub const fn black() -> Color {
		Color::from_rgb(0, 0, 0)
	}

	pub const fn white() -> Color {
		Color::from_rgb(255, 255, 255)
	}

	pub const fn raw(&self) -> u32 {
		self.value
	}

	pub fn blend_mut(&mut self, other: Color) {
		let r = self.r() as u32 * (255 - other.a() as u32) + other.r() as u32 * other.a() as u32;
		let g = self.g() as u32 * (255 - other.a() as u32) + other.g() as u32 * other.a() as u32;
		let b = self.b() as u32 * (255 - other.a() as u32) + other.b() as u32 * other.a() as u32;
		let a = self.a() as u32 * (255 - other.a() as u32) + other.a() as u32 * other.a() as u32;
		self.value = (r << 24) | (g << 16) | (b << 8) | a;
	}
}

impl Display for Color {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.hex())
	}
}
