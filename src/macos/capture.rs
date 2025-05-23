use crate::macos::Monitor;
use crate::{Buffer, PixelFormat, Region};
use core_graphics::display::{CGPoint, CGRect, CGSize};
use core_graphics::image::CGImage;
use std::io;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum CaptureMethod {
    #[default]
    Quartz,
}

pub struct ScreenGrabber {
    monitor: Monitor,
}

impl ScreenGrabber {
    pub fn new(monitor: &Monitor, _capture_type: CaptureMethod) -> io::Result<Self> {
        Ok(Self {
            monitor: monitor.clone(),
        })
    }

    pub fn next_frame<B: Buffer>(&mut self, buf: &mut B) -> io::Result<(usize, u32, u32)> {
        self.next_frame_region_inner(buf, None, PixelFormat::BGRA)
    }

    fn next_frame_region_inner<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
        pixel_format: PixelFormat,
    ) -> io::Result<(usize, u32, u32)> {
        let image = if let Some(region) = region {
            let (full_width, scale_x, full_height, scale_y) = self.monitor.scale_size()?;
            region.check(full_width, full_height)?;
            let rect = CGRect {
                origin: CGPoint {
                    x: region.left as f64 / scale_x,
                    y: region.top as f64 / scale_y,
                },
                size: CGSize {
                    width: region.width as f64 / scale_x,
                    height: region.height as f64 / scale_y,
                },
            };
            self.monitor
                .display()
                .image_for_rect(rect)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no data"))?
        } else {
            self.monitor
                .display()
                .image()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no data"))?
        };
        copy_image_data(&image, buf, pixel_format)
    }

    pub fn next_frame_region<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Region,
    ) -> io::Result<(usize, u32, u32)> {
        self.next_frame_region_inner(buf, Some(region), PixelFormat::BGRA)
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
        CaptureMethod::Quartz
    }
}

/// 复制CGImage的像素数据到buf。返回拷贝字节数。
fn copy_image_data<B: Buffer>(
    image: &CGImage,
    buf: &mut B,
    pixel_format: PixelFormat,
) -> io::Result<(usize, u32, u32)> {
    let data = image.data();
    let width = image.width() as u32;
    let height = image.height() as u32;
    let src_stride = image.bytes_per_row() as u32;
    let stc = data.bytes();
    let len = pixel_format.calc_frame_len(width, height);
    buf.resize(len, 0);
    let dst = buf.as_mut();
    let len = crate::common::convert_bgra(pixel_format, stc, src_stride, dst, width, height)?;
    Ok((len, width, height))
}
