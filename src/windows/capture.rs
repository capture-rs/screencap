use crate::windows::common::Grabber;
use crate::windows::monitor::Monitor;
use crate::windows::{dxgi, gdi, graphics_capture};
use crate::Region;
use std::io;

enum CaptureBackend {
    Graphics(graphics_capture::ScreenGrabber),
    Dxgi(dxgi::ScreenGrabber),
    Gdi(gdi::ScreenGrabber),
}
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum CaptureType {
    Graphics,
    Dxgi,
    Gdi,
    #[default]
    Compatible,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    RGB,
    RGBA,
    BGR,
    #[default]
    BGRA,
}
pub struct ScreenGrabber {
    backend: CaptureBackend,
}

impl ScreenGrabber {
    fn from_monitor(monitor: Monitor) -> io::Result<Self> {
        if let Ok(grabber) = graphics_capture::ScreenGrabber::new(monitor) {
            return Ok(Self {
                backend: CaptureBackend::Graphics(grabber),
            });
        }

        if let Ok(grabber) = dxgi::ScreenGrabber::new(monitor) {
            return Ok(Self {
                backend: CaptureBackend::Dxgi(grabber),
            });
        }

        // fallback to GDI
        let grabber = gdi::ScreenGrabber::new(monitor)?;
        Ok(Self {
            backend: CaptureBackend::Gdi(grabber),
        })
    }
    pub fn new(monitor: Monitor, capture_type: CaptureType) -> io::Result<Self> {
        let backend = match capture_type {
            CaptureType::Graphics => {
                CaptureBackend::Graphics(graphics_capture::ScreenGrabber::new(monitor)?)
            }
            CaptureType::Dxgi => CaptureBackend::Dxgi(dxgi::ScreenGrabber::new(monitor)?),
            CaptureType::Gdi => CaptureBackend::Gdi(gdi::ScreenGrabber::new(monitor)?),
            CaptureType::Compatible => return Self::from_monitor(monitor),
        };
        Ok(Self { backend })
    }

    pub fn next_frame(&mut self, buf: &mut [u8]) -> io::Result<(usize, u32, u32)> {
        match &mut self.backend {
            CaptureBackend::Graphics(g) => g.next_frame(buf),
            CaptureBackend::Dxgi(g) => g.next_frame(buf),
            CaptureBackend::Gdi(g) => g.next_frame(buf),
        }
    }
    pub fn next_frame_region(&mut self, buf: &mut [u8], region: Region) -> io::Result<usize> {
        match &mut self.backend {
            CaptureBackend::Graphics(g) => g.next_frame_region(buf, region),
            CaptureBackend::Dxgi(g) => g.next_frame_region(buf, region),
            CaptureBackend::Gdi(g) => g.next_frame_region(buf, region),
        }
    }
    fn next_frame_region_(
        &mut self,
        buf: &mut [u8],
        region: Option<Region>,
    ) -> io::Result<(usize, u32, u32)> {
        match &mut self.backend {
            CaptureBackend::Graphics(g) => g.next_frame_impl(buf, region),
            CaptureBackend::Dxgi(g) => g.next_frame_impl(buf, region),
            CaptureBackend::Gdi(g) => g.next_frame_impl(buf, region),
        }
    }

    pub fn next_frame_region_format(
        &mut self,
        buf: &mut [u8],
        region: Option<Region>,
        pixel_format: PixelFormat,
    ) -> io::Result<(usize, u32, u32)> {
        let (mut len, width, height) = self.next_frame_region_(buf, region)?;
        match pixel_format {
            PixelFormat::RGB => len = convert_bgra_to_rgb(&mut buf[..len], width, height),
            PixelFormat::RGBA => convert_bgra_to_rgba(&mut buf[..len], width, height),
            PixelFormat::BGR => len = convert_bgra_to_bgr(&mut buf[..len], width, height),
            PixelFormat::BGRA => {}
        }
        Ok((len, width, height))
    }
}

fn convert_bgra_to_bgr(bgra: &mut [u8], width: u32, height: u32) -> usize {
    let pixel_count = width as usize * height as usize;
    let src_len = pixel_count * 4;
    let dst_len = pixel_count * 3;

    assert!(bgra.len() >= src_len);

    let mut write_idx = 0;
    for read_idx in (0..src_len).step_by(4) {
        // 拷贝 B, G, R 三个通道
        bgra[write_idx] = bgra[read_idx]; // B
        bgra[write_idx + 1] = bgra[read_idx + 1]; // G
        bgra[write_idx + 2] = bgra[read_idx + 2]; // R
        write_idx += 3;
    }
    dst_len
}
fn convert_bgra_to_rgba(buf: &mut [u8], width: u32, height: u32) {
    let pixel_count = width as usize * height as usize;
    assert!(buf.len() >= pixel_count * 4);

    for i in 0..pixel_count {
        let base = i * 4;
        buf.swap(base, base + 2); // 交换 B 和 R
    }
}
fn convert_bgra_to_rgb(buf: &mut [u8], width: u32, height: u32) -> usize {
    let pixel_count = width as usize * height as usize;
    let src_len = pixel_count * 4;
    let dst_len = pixel_count * 3;

    assert!(buf.len() >= src_len);

    let mut write = 0;
    for read in (0..src_len).step_by(4) {
        buf[write] = buf[read + 2]; // R
        buf[write + 1] = buf[read + 1]; // G
        buf[write + 2] = buf[read]; // B
        write += 3;
    }

    dst_len
}
