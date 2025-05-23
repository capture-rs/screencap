use crate::windows::monitor::Monitor;
use crate::{Buffer, PixelFormat, Region};
use bytes::BytesMut;
use std::io;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDCW, DeleteDC, DeleteObject,
    GetDIBits, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    SRCCOPY,
};

pub struct ScreenGrabber {
    monitor: Monitor,
    hdc_screen: windows::Win32::Graphics::Gdi::HDC,
    hdc_mem: windows::Win32::Graphics::Gdi::HDC,
    width: u32,
    height: u32,
    hbmp: HBITMAP,
    buffer: BytesMut,
}
impl Drop for ScreenGrabber {
    fn drop(&mut self) {
        unsafe {
            _ = DeleteDC(self.hdc_mem);
            _ = DeleteDC(self.hdc_screen);
            _ = DeleteObject(self.hbmp.into());
        }
    }
}
impl ScreenGrabber {
    pub fn new(monitor: &Monitor) -> io::Result<Self> {
        unsafe {
            let device_name = monitor.device_name_wide()?;
            let hdc_screen = CreateDCW(PCWSTR(device_name.as_ptr()), None, None, None);
            if hdc_screen.is_invalid() {
                return Err(io::Error::last_os_error());
            }
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            if hdc_mem.is_invalid() {
                _ = DeleteDC(hdc_screen);
                return Err(io::Error::last_os_error());
            }
            let (width, height) = monitor.size()?;
            let hbmp = CreateCompatibleBitmap(hdc_screen, width as _, height as _);
            if hbmp.is_invalid() {
                _ = DeleteDC(hdc_mem);
                _ = DeleteDC(hdc_screen);
                return Err(io::Error::last_os_error());
            }
            let old_obj = SelectObject(hdc_mem, hbmp.into());
            if old_obj.is_invalid() {
                _ = DeleteDC(hdc_mem);
                _ = DeleteDC(hdc_screen);
                _ = DeleteObject(hbmp.into());
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                monitor: monitor.clone(),
                hdc_screen,
                hdc_mem,
                width,
                height,
                hbmp,
                buffer: BytesMut::zeroed(width as usize * height as usize * 4),
            })
        }
    }
    fn check_size(&mut self) -> io::Result<()> {
        let (new_width, new_height) = self.monitor.size()?;
        if self.width == new_width && self.height == new_height {
            return Ok(());
        }
        unsafe {
            let hbmp = CreateCompatibleBitmap(self.hdc_screen, new_width as _, new_height as _);
            if hbmp.is_invalid() {
                return Err(io::Error::last_os_error());
            }
            let old_obj = SelectObject(self.hdc_mem, hbmp.into());
            if old_obj.is_invalid() {
                _ = DeleteObject(hbmp.into());
                return Err(io::Error::last_os_error());
            }
            _ = DeleteObject(self.hbmp.into());
            self.hbmp = hbmp;
        }
        self.width = new_width;
        self.height = new_height;
        Ok(())
    }

    pub fn next_frame_impl<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
        pixel_format: PixelFormat,
    ) -> io::Result<(usize, u32, u32)> {
        self.check_size()?;
        let (x, y, width, height) = if let Some(r) = region {
            (r.left, r.top, r.width, r.height)
        } else {
            (0, 0, self.width, self.height)
        };

        unsafe {
            BitBlt(
                self.hdc_mem,
                0,
                0,
                width as i32,
                height as i32,
                Some(self.hdc_screen),
                x as i32,
                y as i32,
                SRCCOPY,
            )?;

            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            self.buffer
                .resize(PixelFormat::BGRA.calc_frame_len(width, height), 0);
            let ptr = self.buffer.as_mut_ptr();
            let res = GetDIBits(
                self.hdc_mem,
                self.hbmp,
                0,
                height,
                Some(ptr as *mut _),
                &mut info,
                DIB_RGB_COLORS,
            );
            if res == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "GetDIBits error",
                ));
            }
            let expected_size = pixel_format.calc_frame_len(width, height);
            buf.resize(expected_size, 0);
            let dst = buf.as_mut();
            let expected_size = crate::common::convert_bgra(
                pixel_format,
                &self.buffer,
                width * 4,
                dst,
                width,
                height,
            )?;
            Ok((expected_size, width, height))
        }
    }
}
