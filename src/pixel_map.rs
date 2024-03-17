use crate::color::Color;
use rapid_qoi::Colors;
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::sync::atomic::{AtomicU32, AtomicUsize};
use std::sync::Arc;
use std::thread;

pub(crate) struct PixelMap {
    pixels: Vec<AtomicU32>,
    width: AtomicU32,
    height: AtomicU32,
    version: AtomicUsize,
}

impl PixelMap {
    pub fn new(width: u32, height: u32) -> PixelMap {
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for _ in 0..width {
            for _ in 0..height {
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

    pub fn load_image(filename: &str) -> PixelMap {
        let img = match std::fs::read(filename) {
            Ok(x) => x,
            Err(_) => return PixelMap::new(1280, 720),
        };
        let qoi = match rapid_qoi::Qoi::decode_alloc(&img) {
            Ok(x) => x,
            Err(_) => return PixelMap::new(1280, 720),
        };
        let (width, height) = (qoi.0.width, qoi.0.height);
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let color = match qoi.0.colors {
                    Colors::Rgb => Color::from_rgb(
                        qoi.1[(x + y * width) as usize * 3],
                        qoi.1[(x + y * width) as usize * 3 + 1],
                        qoi.1[(x + y * width) as usize * 3 + 2],
                    ),
                    Colors::Rgba => Color::from_rgba(
                        qoi.1[(x + y * width) as usize * 4],
                        qoi.1[(x + y * width) as usize * 4 + 1],
                        qoi.1[(x + y * width) as usize * 4 + 2],
                        qoi.1[(x + y * width) as usize * 4 + 3],
                    ),
                    _ => Color::black(),
                };
                pixels.push(AtomicU32::new(color.raw()));
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
        self.version.fetch_add(1, SeqCst);
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

    pub fn to_qoi(&self) -> Arc<Box<[u8]>> {
        let w = self.get_width();
        let h = self.get_height();
        let mut buf = Vec::with_capacity((w * h * 4) as usize);
        self.pixels
            .iter()
            .for_each(|x| Color::new(x.load(Relaxed)).add_to_vec(&mut buf));
        let qoi = rapid_qoi::Qoi {
            width: w,
            height: h,
            colors: Colors::Rgba,
        };
        let qoi_buffer = qoi.encode_alloc(&buf).unwrap();

        let qoi_arc = Arc::new(qoi_buffer.into_boxed_slice());
        let t_arc = Arc::clone(&qoi_arc);
        thread::spawn(move || {
            std::fs::write("image.qoi", &*t_arc).unwrap();
        });

        qoi_arc
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
