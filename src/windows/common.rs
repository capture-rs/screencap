use crate::{Buffer, Region};
use std::io;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_BOX, D3D11_CPU_ACCESS_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};

pub trait Grabber {
    fn get_device(&self) -> &ID3D11Device;
    fn get_context(&self) -> &ID3D11DeviceContext;
    fn get_desc(
        &self,
        texture: &ID3D11Texture2D,
        region: Option<Region>,
    ) -> io::Result<(D3D11_TEXTURE2D_DESC, u32, u32)> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe {
            texture.GetDesc(&mut desc);
        }
        let full_width = desc.Width;
        let full_height = desc.Height;

        desc.Usage = D3D11_USAGE_STAGING;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.BindFlags = 0;
        desc.MiscFlags = 0;
        if let Some(region) = region {
            region.check(full_width, full_height)?;
            desc.Width = region.width;
            desc.Height = region.height;
        }
        Ok((desc, full_width, full_height))
    }
    fn create_texture2d(&self, desc: D3D11_TEXTURE2D_DESC) -> io::Result<ID3D11Texture2D> {
        unsafe {
            let mut tex = None;
            self.get_device()
                .CreateTexture2D(&desc, None, Some(&mut tex))?;
            tex.ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Failed to create staging texture")
            })
        }
    }

    fn next_frame<B: Buffer>(&mut self, buf: &mut B) -> io::Result<(usize, u32, u32)> {
        self.next_frame_impl(buf, None)
    }
    fn next_frame_impl<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
    ) -> io::Result<(usize, u32, u32)>;
    fn copy_resource<B: Buffer>(
        &self,
        staging: ID3D11Texture2D,
        texture: &ID3D11Texture2D,
        region: Option<Region>,
        full_width: u32,
        full_height: u32,
        buf: &mut B,
    ) -> io::Result<(usize, u32, u32)> {
        const PIXEL_WIDTH: usize = 4;
        unsafe {
            let (target_width, target_height, staging) = if let Some(Region {
                left,
                top,
                width,
                height,
            }) = region
            {
                // 区域拷贝
                let box_region = D3D11_BOX {
                    left,
                    top,
                    front: 0,
                    right: left + width,
                    bottom: top + height,
                    back: 1,
                };
                self.get_context().CopySubresourceRegion(
                    &staging,
                    0,
                    0,
                    0,
                    0,
                    texture,
                    0,
                    Some(&box_region),
                );
                (width, height, staging)
            } else {
                self.get_context().CopyResource(&staging, texture);
                (full_width, full_height, staging)
            };

            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.get_context()
                .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

            let row_pitch = mapped.RowPitch as usize;
            let expected_size = target_height as usize * target_width as usize * PIXEL_WIDTH;
            buf.resize(expected_size, 0);
            let buf = buf.as_mut();
            if buf.len() < expected_size {
                self.get_context().Unmap(&staging, 0);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Buffer is too small to hold the frame data",
                ));
            }
            if expected_size as u32 == mapped.DepthPitch {
                std::ptr::copy_nonoverlapping(
                    mapped.pData as *const u8,
                    buf.as_mut_ptr(),
                    expected_size,
                )
            } else {
                // 拷贝每一行（确保跳过 row_pitch 的填充字节）
                let bytes_per_row = target_width as usize * PIXEL_WIDTH;
                for y in 0..target_height as usize {
                    let src = (mapped.pData as *const u8).add(y * row_pitch);
                    let dst = buf.as_mut_ptr().add(y * bytes_per_row);
                    std::ptr::copy_nonoverlapping(src, dst, bytes_per_row);
                }
            }

            self.get_context().Unmap(&staging, 0);
            Ok((expected_size, target_width, target_height))
        }
    }
}
