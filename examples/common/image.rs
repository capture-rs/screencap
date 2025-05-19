use image::RgbImage;
use std::path::Path;

pub fn save_to_file(width: u32, height: u32, buf: &[u8]) {
    let mut data = Vec::with_capacity((width * height * 3) as usize);

    for y in 0..height {
        for x in 0..width {
            let pixel = y as usize * width as usize * 4 + (x * 4) as usize;
            // 读取 BGRA 数据
            let b = buf[pixel];
            let g = buf[pixel + 1];
            let r = buf[pixel + 2];
            let _a = buf[pixel + 3];
            // 进行 BGRA 到 RGB 的转换
            data.push(r);
            data.push(g);
            data.push(b);
        }
    }
    let image = RgbImage::from_raw(width, height, data).expect("Failed to create image");
    let path = Path::new("screenshot.jpg");
    image.save(path).expect("Failed to save image");
}
