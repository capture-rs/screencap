use screencap::{CaptureType, Monitor};
use std::io;
mod common;

fn main() -> io::Result<()> {
    let list = Monitor::all()?;
    for x in list {
        println!("{x:?},{:?}", x.size())
    }
    let monitor = Monitor::primary()?;
    let mut grabber = screencap::ScreenGrabber::new(&monitor, CaptureType::default())?;
    // 避免第一帧黑帧
    std::thread::sleep(std::time::Duration::from_millis(100));
    let (width, height) = monitor.size()?;
    let mut buf = vec![0; (width * height * 4) as usize];

    let (len, width, height) = grabber.next_frame(&mut buf)?;
    common::image::save_to_file(width, height, &buf[..len]);
    Ok(())
}
