use crate::windows::common::Grabber;
use crate::windows::monitor::Monitor;
use crate::{Buffer, Region};
use std::io;
use std::ops::Deref;
use std::sync::mpsc::{self, Receiver};
use windows::core::IInspectable;
use windows::Graphics::Capture::Direct3D11CaptureFrame;
use windows::Graphics::SizeInt32;
use windows::{
    core::Interface,
    Foundation::TypedEventHandler,
    Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession},
    Graphics::DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0},
            Direct3D11::*,
            Dxgi::IDXGIDevice,
        },
        System::WinRT::{
            Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
            Graphics::Capture::IGraphicsCaptureItemInterop, RoInitialize, RO_INIT_MULTITHREADED,
        },
    },
};

pub struct ScreenGrabber {
    id: i64,
    device: ID3D11Device,
    receiver: Receiver<()>,

    session: GraphicsCaptureSession,
    frame_pool: Direct3D11CaptureFramePool,
    context: ID3D11DeviceContext,
    size: SizeInt32,
}

impl Drop for ScreenGrabber {
    fn drop(&mut self) {
        if let Err(e) = self.session.Close() {
            eprintln!("Error closing session: {}", e);
        }

        if let Err(e) = self.frame_pool.RemoveFrameArrived(self.id) {
            eprintln!("Error removing frame arrived handler: {}", e);
        }

        if let Err(e) = self.frame_pool.Close() {
            eprintln!("Error closing frame pool: {}", e);
        }
    }
}

impl ScreenGrabber {
    pub fn is_supported() -> io::Result<bool> {
        Ok(GraphicsCaptureSession::IsSupported()?)
    }

    pub fn new(monitor: &Monitor) -> io::Result<Self> {
        if !Self::is_supported()? {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Unsupported GraphicsCapture",
            ));
        }
        unsafe {
            // Initialize the COM library
            RoInitialize(RO_INIT_MULTITHREADED)?;

            // Create the D3D11 device and context
            let (device, context) = create_d3d11_device()?;

            // Create Direct3D device
            let d3d_device = create_direct3d_device(&device)?;

            // Create capture item for monitor
            let item = create_capture_item_for_monitor(monitor)?;
            let size = item.Size()?;

            // Create the capture frame pool
            let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                &d3d_device,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                3,
                size,
            )?;
            d3d_device.Close()?;
            // Create the capture session and start capture
            let session = frame_pool.CreateCaptureSession(&item)?;
            session.StartCapture()?;

            let (tx, rx) = mpsc::sync_channel(1);
            let frame_pool_clone = frame_pool.clone();

            // Register event handler for frame arrival
            let id = frame_pool_clone.FrameArrived(&TypedEventHandler::<
                Direct3D11CaptureFramePool,
                IInspectable,
            >::new(move |_, _| {
                _ = tx.try_send(());
                Ok(())
            }))?;

            Ok(Self {
                id,
                device,
                context,
                receiver: rx,
                session,
                frame_pool,
                size,
            })
        }
    }
    fn next_texture(&mut self) -> io::Result<(ID3D11Texture2D, OwnedDirect3D11CaptureFrame)> {
        loop {
            match self.frame_pool.TryGetNextFrame() {
                Ok(frame) => unsafe {
                    let frame = OwnedDirect3D11CaptureFrame(frame);
                    let content_size = frame.ContentSize()?;

                    if content_size != self.size {
                        let d3d_device = create_direct3d_device(&self.device)?;
                        self.frame_pool.Recreate(
                            &d3d_device,
                            DirectXPixelFormat::B8G8R8A8UIntNormalized,
                            3,
                            content_size,
                        )?;
                        d3d_device.Close()?;
                        self.size = content_size;
                    }

                    let surface = frame.Surface()?;
                    let access: windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess = surface.cast()?;
                    let texture: ID3D11Texture2D = access.GetInterface()?;
                    return Ok((texture, frame));
                },
                Err(e) => {
                    e.code().ok()?;
                    self.receiver
                        .recv()
                        .map_err(|_e| io::Error::new(io::ErrorKind::Other, "channel closed"))?;
                }
            }
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
    fn next_frame_impl<B: Buffer>(
        &mut self,
        buf: &mut B,
        region: Option<Region>,
    ) -> io::Result<(usize, u32, u32)> {
        let (texture, _guard) = self.next_texture()?;
        let (desc, full_width, full_height) = self.get_desc(&texture, region)?;
        let staging = self.create_texture2d(desc)?;
        self.copy_resource(staging, &texture, region, full_width, full_height, buf)
    }
}
struct OwnedDirect3D11CaptureFrame(Direct3D11CaptureFrame);
impl Drop for OwnedDirect3D11CaptureFrame {
    fn drop(&mut self) {
        _ = self.0.Close();
    }
}
impl Deref for OwnedDirect3D11CaptureFrame {
    type Target = Direct3D11CaptureFrame;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn create_d3d11_device() -> io::Result<(ID3D11Device, ID3D11DeviceContext)> {
    let mut device = None;
    let mut context = None;

    unsafe {
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
    }

    Ok((
        device.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to create device"))?,
        context.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to create context"))?,
    ))
}

fn create_direct3d_device(device: &ID3D11Device) -> io::Result<IDirect3DDevice> {
    unsafe {
        let dxgi_device: IDXGIDevice = device.cast()?;
        let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
        Ok(inspectable.cast()?)
    }
}

fn create_capture_item_for_monitor(monitor: &Monitor) -> io::Result<GraphicsCaptureItem> {
    unsafe {
        let h_monitor = monitor.h_monitor();
        let interop: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        Ok(interop.CreateForMonitor(h_monitor)?)
    }
}
