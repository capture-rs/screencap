use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, MonitorFromPoint, DEVMODEW,
    DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, HDC, HMONITOR, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

#[derive(Clone, Debug)]
pub struct Monitor {
    index: u32,
    h_monitor: HMONITOR,
}

impl Monitor {
    pub fn primary() -> io::Result<Self> {
        let h_monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        if h_monitor.is_invalid() {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Primary monitor not found",
            ))
        } else {
            Ok(Self {
                index: 0,
                h_monitor,
            })
        }
    }

    pub fn from_index(index: u32) -> io::Result<Self> {
        let monitors = Self::all()?;
        monitors
            .get(index as usize)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Monitor index out of range"))
    }

    pub fn all() -> io::Result<Vec<Self>> {
        extern "system" fn monitor_enum_proc(
            h_monitor: HMONITOR,
            _: HDC,
            _: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let monitors = unsafe { &mut *(lparam.0 as *mut Vec<Monitor>) };
            let index = monitors.len() as u32;
            monitors.push(Monitor { index, h_monitor });
            true.into()
        }

        let mut monitors = Vec::<Monitor>::new();
        let lparam = LPARAM(&mut monitors as *mut _ as isize);
        let success = unsafe { EnumDisplayMonitors(None, None, Some(monitor_enum_proc), lparam) };
        if !success.as_bool() {
            return Err(io::Error::last_os_error());
        }
        Ok(monitors)
    }
    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn h_monitor(&self) -> HMONITOR {
        self.h_monitor
    }

    pub fn size(&self) -> io::Result<(u32, u32)> {
        let name = self.device_name_wide()?;
        unsafe {
            let mut device_mode = DEVMODEW {
                dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                ..Default::default()
            };
            if !EnumDisplaySettingsW(
                PCWSTR(name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut device_mode,
            )
            .as_bool()
            {
                return Err(io::Error::last_os_error());
            }
            Ok((device_mode.dmPelsWidth, device_mode.dmPelsHeight))
        }
    }
    pub fn device_name_wide(&self) -> io::Result<Vec<u16>> {
        let device_name = self.device_name()?;
        let wide = OsStr::new(&device_name)
            .encode_wide()
            .chain(Some(0))
            .collect();
        Ok(wide)
    }
    pub fn device_name(&self) -> io::Result<String> {
        unsafe {
            let mut device = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };

            if !EnumDisplayDevicesW(PCWSTR::null(), self.index, &mut device, 0).as_bool() {
                return Err(io::Error::last_os_error());
            }

            let len = device
                .DeviceName
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(device.DeviceName.len());
            let os_str = OsString::from_wide(&device.DeviceName[..len]);
            Ok(os_str.to_string_lossy().into_owned())
        }
    }
}

#[derive(Clone, Debug)]
pub struct VirtualScreen {}
impl VirtualScreen {
    pub fn new() -> io::Result<Self> {
        Ok(VirtualScreen {})
    }
    pub fn rect(&self) -> io::Result<(i32, i32, u32, u32)> {
        let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let (width, height) = self.size()?;
        Ok((left, top, width, height))
    }

    pub fn size(&self) -> io::Result<(u32, u32)> {
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

        if width <= 0 || height <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to retrieve virtual screen dimensions",
            ));
        }
        Ok((width as u32, height as u32))
    }
}
