# screencap
Capture screen data

```rust
use screencap::{CaptureType, Monitor, Region, PixelFormat};
use std::io;

fn main() -> io::Result<()> {
    let list = Monitor::all()?;
    for x in list {
        println!("{x:?},{:?}", x.size())
    }
    let monitor = Monitor::primary()?;
    let mut grabber = screencap::ScreenGrabber::new(monitor, CaptureType::Graphics)?;
    // 如果使用Dxgi，则需要等一会避免第一帧黑帧
    std::thread::sleep(std::time::Duration::from_millis(100));
    let (width, height) = monitor.size()?;
    // 截取屏幕左上角
    let width = width / 2;
    let height = height / 2;
    let region = Region {
        left: 0,
        top: 0,
        width,
        height,
    };

    let mut buf = vec![0; (width * height * 4) as usize];
    // 获取BGRA数据
    let len = grabber.next_frame_region(&mut buf, region)?;
    // 获取RGB数据
    let (len, width, height) = grabber.next_frame_region(&mut buf, Some(region), PixelFormat::RGB)?;
    Ok(())
}
```