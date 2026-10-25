//! The three baseline software formats from libretro.h, with row padding respected.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Frame {
    /// libretro SET_ROTATION counts quarter turns counter-clockwise. Rotate here so
    /// the player, pause preview and save thumbnail all see the same upright image.
    pub fn rotated(self, turns: u32) -> Self {
        let turns = turns % 4;
        if turns == 0 {
            return self;
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let (nw, nh) = if turns % 2 == 1 { (h, w) } else { (w, h) };
        let mut rgba = vec![0; self.rgba.len()];
        for y in 0..h {
            for x in 0..w {
                let (nx, ny) = match turns {
                    1 => (y, w - 1 - x),
                    2 => (w - 1 - x, h - 1 - y),
                    _ => (h - 1 - y, x),
                };
                rgba[(ny * nw + nx) * 4..][..4].copy_from_slice(&self.rgba[(y * w + x) * 4..][..4]);
            }
        }
        Self {
            width: nw as u32,
            height: nh as u32,
            rgba,
        }
    }
    pub fn decode(
        data: &[u8],
        width: u32,
        height: u32,
        pitch: usize,
        format: u32,
    ) -> Result<Self, String> {
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > 16_777_216
        {
            return Err("invalid software frame geometry".into());
        }
        let bpp = match format {
            0 | 2 => 2,
            1 => 4,
            _ => return Err("unsupported pixel format".into()),
        };
        let row = width as usize * bpp;
        if pitch < row
            || data.len()
                < pitch
                    .saturating_mul(height as usize - 1)
                    .saturating_add(row)
        {
            return Err("software frame has a short pitch or buffer".into());
        }
        let mut rgba = vec![0; width as usize * height as usize * 4];
        let five = |n: u16| ((n & 31) * 255 / 31) as u8;
        let six = |n: u16| ((n & 63) * 255 / 63) as u8;
        for y in 0..height as usize {
            for x in 0..width as usize {
                let at = y * pitch + x * bpp;
                let rgb = if format == 1 {
                    let p = u32::from_ne_bytes(data[at..at + 4].try_into().unwrap());
                    [(p >> 16) as u8, (p >> 8) as u8, p as u8]
                } else {
                    let p = u16::from_ne_bytes([data[at], data[at + 1]]);
                    if format == 0 {
                        [five(p >> 10), five(p >> 5), five(p)]
                    } else {
                        [five(p >> 11), six(p >> 5), five(p)]
                    }
                };
                rgba[(y * width as usize + x) * 4..][..4]
                    .copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_preserves_pixels_in_every_direction() {
        let frame = Frame {
            width: 3,
            height: 2,
            rgba: (1..=6).flat_map(|x| [x, 0, 0, 255]).collect(),
        };
        for (turns, expected) in [
            (1, vec![3, 6, 2, 5, 1, 4]),
            (2, vec![6, 5, 4, 3, 2, 1]),
            (3, vec![4, 1, 5, 2, 6, 3]),
        ] {
            let rotated = frame.clone().rotated(turns);
            assert_eq!(
                (rotated.width, rotated.height),
                if turns % 2 == 1 { (2, 3) } else { (3, 2) }
            );
            assert_eq!(
                rotated
                    .rgba
                    .chunks_exact(4)
                    .map(|p| p[0])
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(rotated.rotated(4 - turns), frame);
        }
    }
    #[test]
    fn formats_and_padding() {
        for (format, pixel) in [(0, 0x7C00u32), (1, 0x00FF0000), (2, 0xF800)] {
            let bpp = if format == 1 { 4 } else { 2 };
            let mut data = vec![0xAB; 16];
            for offset in [0, 8] {
                data[offset..offset + bpp].copy_from_slice(&pixel.to_ne_bytes()[..bpp]);
            }
            let frame = Frame::decode(&data, 1, 2, 8, format).unwrap();
            assert_eq!(frame.rgba, [255, 0, 0, 255, 255, 0, 0, 255]);
        }
        assert!(Frame::decode(&[0; 8], 2, 2, 1, 2).is_err());
        assert!(Frame::decode(&[], u32::MAX, 2, usize::MAX, 2).is_err());
    }
}
