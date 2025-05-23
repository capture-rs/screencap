use crate::common::*;
use crate::windows::common::Grabber;
use crate::windows::monitor::Monitor;
use crate::windows::{dxgi, gdi, graphics_capture};
use crate::{PixelFormat, Region};
use std::io;

enum CaptureBackend {
    Graphics(graphics_capture::ScreenGrabber),
    Dxgi(dxgi::ScreenGrabber),
    Gdi(gdi::ScreenGrabber),
}
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum CaptureMethod {
    Graphics,
    Dxgi,
    Gdi,
    #[default]
    Compatible,
}

pub struct ScreenGrabber {
    backend: CaptureBackend,
}

impl ScreenGrabber {
    fn from_monitor(monitor: &Monitor) -> io::Result<Self> {
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
    pub fn new(monitor: &Monitor, capture_type: CaptureMethod) -> io::Result<Self> {
        let backend = match capture_type {
            CaptureMethod::Graphics => {
                CaptureBackend::Graphics(graphics_capture::ScreenGrabber::new(monitor)?)
            }
            CaptureMethod::Dxgi => CaptureBackend::Dxgi(dxgi::ScreenGrabber::new(monitor)?),
            CaptureMethod::Gdi => CaptureBackend::Gdi(gdi::ScreenGrabber::new(monitor)?),
            CaptureMethod::Compatible => return Self::from_monitor(monitor),
        };
        Ok(Self { backend })
    }

    pub fn next_frame<B: Buffer>(&mut self, buf: &mut B) -> io::Result<(usize, u32, u32)> {
        self.next_frame_region_inner(buf, None, PixelFormat::BGRA)
    }
    pub fn next_frame_region<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Region,
    ) -> io::Result<(usize, u32, u32)> {
        self.next_frame_region_inner(buf, Some(region), PixelFormat::BGRA)
    }
    fn next_frame_region_inner<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
        pixel_format: PixelFormat,
    ) -> io::Result<(usize, u32, u32)> {
        match &mut self.backend {
            CaptureBackend::Graphics(g) => g.next_frame_impl(buf, region, pixel_format),
            CaptureBackend::Dxgi(g) => g.next_frame_impl(buf, region, pixel_format),
            CaptureBackend::Gdi(g) => g.next_frame_impl(buf, region, pixel_format),
        }
    }

    pub fn next_frame_region_format<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
        pixel_format: PixelFormat,
    ) -> io::Result<(usize, u32, u32)> {
        self.next_frame_region_inner(buf, region, pixel_format)
    }
    pub fn capture_method(&self) -> CaptureMethod {
        match &self.backend {
            CaptureBackend::Graphics(_) => CaptureMethod::Graphics,
            CaptureBackend::Dxgi(_) => CaptureMethod::Dxgi,
            CaptureBackend::Gdi(_) => CaptureMethod::Gdi,
        }
    }
}
