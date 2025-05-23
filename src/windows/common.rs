use crate::{Buffer, PixelFormat, Region};
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

    fn next_frame_impl<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
        pixel_format: PixelFormat,
    ) -> io::Result<(usize, u32, u32)>;
    #[allow(clippy::too_many_arguments)]
    fn copy_resource<B: Buffer>(
        &self,
        staging: ID3D11Texture2D,
        texture: &ID3D11Texture2D,
        region: Option<Region>,
        full_width: u32,
        full_height: u32,
        buf: &mut B,
        pixel_format: PixelFormat,
    ) -> io::Result<(usize, u32, u32)> {
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
            let expected_size = pixel_format.calc_frame_len(target_width, target_height);
            buf.resize(expected_size, 0);
            let buf = buf.as_mut();

            let src =
                std::slice::from_raw_parts(mapped.pData as *const u8, mapped.DepthPitch as usize);
            let rs = crate::common::convert_bgra(
                pixel_format,
                src,
                row_pitch as _,
                buf,
                target_width,
                target_height,
            );

            self.get_context().Unmap(&staging, 0);
            Ok((rs?, target_width, target_height))
        }
    }
}
