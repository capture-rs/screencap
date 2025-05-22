use image::RgbImage;
use screencap::{CaptureMethod, Monitor, PixelFormat, Region};
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    let list = Monitor::all()?;
    for x in list {
        println!("{x:?},{:?}", x.size())
    }
    let monitor = Monitor::primary()?;
    let mut grabber = screencap::ScreenGrabber::new(&monitor, CaptureMethod::default())?;
    // 避免第一帧黑帧
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
    let mut buf = Vec::new();
    let (len, width, height) =
        grabber.next_frame_region_format(&mut buf, Some(region), PixelFormat::RGB)?;
    // 注意，返回的实际width、height并不一定等于region中的width、height
    println!("next_frame_region_format {:?},{:?}", width, height);
    let image =
        RgbImage::from_raw(width, height, buf[..len].to_vec()).expect("Failed to create image");
    let path = Path::new("screenshot.jpg");
    image.save(path).expect("Failed to save image");
    Ok(())
}
