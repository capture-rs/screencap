use std::io;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Region {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}
impl Region {
    pub fn new(left: u32, top: u32, width: u32, height: u32) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }
    pub fn check(&self, full_width: u32, full_height: u32) -> io::Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "width or height cannot be 0",
            ));
        }
        if self.left + self.width > full_width {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Region out of bounds: left({}) + width({}) > full_width({full_width})",
                    self.left, self.width
                ),
            ));
        }
        if self.top + self.height > full_height {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Region out of bounds: top({}) + height({}) > full_height({full_height})",
                    self.top, self.height
                ),
            ));
        }
        Ok(())
    }
}
