use screencap::{CaptureMethod, Monitor, VirtualScreen};
use std::io;
mod common;

fn main() -> io::Result<()> {
    let screen = VirtualScreen::new()?;
    println!("{:?}", screen.rect());
    let list = Monitor::all()?;
    for x in list {
        println!("{x:?},{:?}", x.size())
    }
    let monitor = Monitor::primary()?;
    let mut grabber = screencap::ScreenGrabber::new(&monitor, CaptureMethod::default())?;
    // 避免第一帧黑帧
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut buf = Vec::new();

    let (len, width, height) = grabber.next_frame(&mut buf)?;
    common::image::save_to_file(width, height, &buf[..len]);
    Ok(())
}
