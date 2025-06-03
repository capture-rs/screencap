use crate::windows::common::Grabber;
use crate::windows::monitor::Monitor;
use crate::Region;
use std::io;
use windows::core::Interface;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIDevice, IDXGIOutput1, IDXGIOutputDuplication, DXGI_OUTDUPL_FRAME_INFO,
};

pub struct ScreenGrabber {
    duplication: IDXGIOutputDuplication,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
}

impl ScreenGrabber {
    pub fn new(monitor: &Monitor) -> io::Result<Self> {
        unsafe {
            let mut device = None;
            let mut context = None;

            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;

            let device = device.ok_or_else(|| io::Error::other("No D3D11 device"))?;
            let context = context.ok_or_else(|| io::Error::other("No D3D11 context"))?;

            let dxgi_device: IDXGIDevice = device.cast()?;
            let adapter = dxgi_device.GetAdapter()?;
            let output = adapter.EnumOutputs(monitor.index())?;
            let output1: IDXGIOutput1 = output.cast()?;

            let duplication = output1.DuplicateOutput(&device)?;
            Ok(Self {
                duplication,
                device,
                context,
            })
        }
    }
    fn next_texture(&mut self) -> io::Result<ID3D11Texture2D> {
        unsafe {
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut desktop_resource = None;

            self.duplication
                .AcquireNextFrame(500, &mut frame_info, &mut desktop_resource)?;

            let desktop_resource = desktop_resource
                .ok_or_else(|| io::Error::other("Failed to acquire desktop resource"))?;

            let desktop_texture: ID3D11Texture2D = desktop_resource.cast()?;
            Ok(desktop_texture)
        }
    }
}
impl Grabber for ScreenGrabber {
    fn get_device(&self) -> &ID3D11Device {
        &self.device
    }

    fn get_context(&self) -> &ID3D11DeviceContext {
        &self.context
    }
    fn next_frame_impl(
        &mut self,
        buf: &mut [u8],
        region: Option<Region>,
    ) -> io::Result<(usize, u32, u32)> {
        let texture = self.next_texture();
        let _guard = DxgiFrameGuard(&self.duplication);
        let texture = texture?;
        let (desc, full_width, full_height) = self.get_desc(&texture, region)?;
        let staging = self.create_texture2d(desc)?;
        self.copy_resource(staging, &texture, region, full_width, full_height, buf)
    }
}
struct DxgiFrameGuard<'a>(&'a IDXGIOutputDuplication);
impl Drop for DxgiFrameGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.ReleaseFrame();
        }
    }
}
