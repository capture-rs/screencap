use crate::common::*;
use crate::windows::common::Grabber;
use crate::windows::monitor::Monitor;
use crate::windows::{dxgi, gdi, graphics_capture};
use crate::{PixelFormat, Region, VirtualScreen};
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
    pub fn new_virtual(
        virtual_screen: &VirtualScreen,
        capture_type: CaptureMethod,
    ) -> io::Result<ScreenGrabber> {
        let backend = match capture_type {
            CaptureMethod::Graphics => {
                todo!()
            }
            CaptureMethod::Dxgi => todo!(),
            CaptureMethod::Gdi => {
                CaptureBackend::Gdi(gdi::ScreenGrabber::new_virtual(virtual_screen)?)
            }
            CaptureMethod::Compatible => todo!(),
        };
        Ok(Self { backend })
    }
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
        match &mut self.backend {
            CaptureBackend::Graphics(g) => g.next_frame(buf),
            CaptureBackend::Dxgi(g) => g.next_frame(buf),
            CaptureBackend::Gdi(g) => g.next_frame(buf),
        }
    }
    pub fn next_frame_region<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Region,
    ) -> io::Result<(usize, u32, u32)> {
        self.next_frame_region_inner(buf, Some(region))
    }
    fn next_frame_region_inner<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
    ) -> io::Result<(usize, u32, u32)> {
        match &mut self.backend {
            CaptureBackend::Graphics(g) => g.next_frame_impl(buf, region),
            CaptureBackend::Dxgi(g) => g.next_frame_impl(buf, region),
            CaptureBackend::Gdi(g) => g.next_frame_impl(buf, region),
        }
    }

    pub fn next_frame_region_format<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
        pixel_format: PixelFormat,
    ) -> io::Result<(usize, u32, u32)> {
        let (mut len, width, height) = self.next_frame_region_inner(buf, region)?;
        let buf = buf.as_mut();
        match pixel_format {
            PixelFormat::RGB => len = convert_bgra_to_rgb(&mut buf[..len], width, height),
            PixelFormat::RGBA => convert_bgra_to_rgba(&mut buf[..len], width, height),
            PixelFormat::BGR => len = convert_bgra_to_bgr(&mut buf[..len], width, height),
            PixelFormat::BGRA => {}
        }
        Ok((len, width, height))
    }
    pub fn capture_method(&self) -> CaptureMethod {
        match &self.backend {
            CaptureBackend::Graphics(_) => CaptureMethod::Graphics,
            CaptureBackend::Dxgi(_) => CaptureMethod::Dxgi,
            CaptureBackend::Gdi(_) => CaptureMethod::Gdi,
        }
    }
}
