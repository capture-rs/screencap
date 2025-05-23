use bytes::BytesMut;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    RGB,
    RGBA,
    BGR,
    #[default]
    BGRA,
    YUV420,
}
impl PixelFormat {
    pub fn calc_frame_len(&self, width: u32, height: u32) -> usize {
        match self {
            PixelFormat::RGB | PixelFormat::BGR => width as usize * height as usize * 3,
            PixelFormat::RGBA | PixelFormat::BGRA => width as usize * height as usize * 4,
            PixelFormat::YUV420 => (width as usize) * (height as usize) * 3 / 2,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Region {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}
impl Region {
    pub fn new(left: u32, top: u32, width: u32, height: u32) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }
    pub fn check(&self, full_width: u32, full_height: u32) -> io::Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "width or height cannot be 0",
            ));
        }
        if self.left + self.width > full_width {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Region out of bounds: left({}) + width({}) > full_width({full_width})",
                    self.left, self.width
                ),
            ));
        }
        if self.top + self.height > full_height {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Region out of bounds: top({}) + height({}) > full_height({full_height})",
                    self.top, self.height
                ),
            ));
        }
        Ok(())
    }
}

pub(crate) fn convert_bgra(
    pixel_format: PixelFormat,
    src: &[u8],
    src_stride: u32,
    dst: &mut [u8],
    width: u32,
    height: u32,
) -> io::Result<usize> {
    let len = pixel_format.calc_frame_len(width, height);
    if dst.len() < len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Buffer is too small to hold the frame data",
        ));
    }
    match pixel_format {
        PixelFormat::RGB => {
            yuv::bgra_to_rgb(src, src_stride, dst, width * 3, width, height)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
        PixelFormat::RGBA => {
            yuv::bgra_to_rgba(src, src_stride, dst, width * 4, width, height)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
        PixelFormat::BGR => {
            yuv::bgra_to_bgr(src, src_stride, dst, width * 3, width, height)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
        PixelFormat::BGRA => {
            if src_stride == width * 4 {
                dst[..len].copy_from_slice(src)
            } else {
                // 拷贝每一行（确保跳过 src_stride 的填充字节）
                let bytes_per_row = width as usize * 4;
                for y in 0..height as usize {
                    let src = &src[y * src_stride as usize..];
                    let dst = &mut dst[y * bytes_per_row..];
                    dst[..bytes_per_row].copy_from_slice(&src[..bytes_per_row])
                }
            }
        }
        PixelFormat::YUV420 => {
            let width = width as usize;
            let height = height as usize;
            let y_size = width * height;
            let uv_size = (width / 2) * (height / 2);

            let (y_plane, uv) = dst[..y_size + 2 * uv_size].split_at_mut(y_size);
            let (u_plane, v_plane) = uv.split_at_mut(uv_size);

            let mut planar = yuv::YuvPlanarImageMut {
                y_plane: yuv::BufferStoreMut::Borrowed(y_plane),
                y_stride: width as u32,
                u_plane: yuv::BufferStoreMut::Borrowed(u_plane),
                u_stride: (width / 2) as u32,
                v_plane: yuv::BufferStoreMut::Borrowed(v_plane),
                v_stride: (width / 2) as u32,
                width: width as u32,
                height: height as u32,
            };
            yuv::bgra_to_yuv420(
                &mut planar,
                src,
                src_stride,
                yuv::YuvRange::Limited,
                yuv::YuvStandardMatrix::Bt601,
                yuv::YuvConversionMode::default(),
            )
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
    }
    Ok(len)
}
pub trait Buffer: AsMut<[u8]> + AsRef<[u8]> {
    fn resize(&mut self, _new_len: usize, _value: u8) {}
}
impl Buffer for Vec<u8> {
    fn resize(&mut self, new_len: usize, value: u8) {
        Vec::<u8>::resize(self, new_len, value);
    }
}
impl Buffer for &mut Vec<u8> {
    fn resize(&mut self, new_len: usize, value: u8) {
        Vec::<u8>::resize(self, new_len, value);
    }
}
impl Buffer for BytesMut {
    fn resize(&mut self, new_len: usize, value: u8) {
        BytesMut::resize(self, new_len, value);
    }
}
impl Buffer for &mut BytesMut {
    fn resize(&mut self, new_len: usize, value: u8) {
        BytesMut::resize(self, new_len, value);
    }
}
impl Buffer for &mut [u8] {}
impl Buffer for [u8] {}
