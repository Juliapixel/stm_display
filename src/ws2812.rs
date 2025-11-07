use core::ptr::NonNull;

use embedded_graphics::{
    pixelcolor::raw::RawU24,
    prelude::{Dimensions, DrawTarget, PixelColor, Point, Size},
    primitives::Rectangle,
};

use crate::fmt::assert;

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
pub struct WireColor([u8; 3]);

impl WireColor {
    pub const fn new(value: u8) -> Self {
        let mut inner = [0, 0, 0];
        let mut vshift: usize = 7;
        let mut bshift: usize = 0;
        while vshift <= 7 {
            let high = value >> vshift & 1 == 1;
            let b: u8 = if high { 0b110_00000 } else { 0b100_00000 };
            if bshift == 5 && high {
                inner[2] |= 0b1000_0000
            }
            inner[[0, 0, 0, 1, 1, 1, 2, 2][bshift]] |= b >> ((bshift * 3) % 8);
            vshift = vshift.wrapping_sub(1);
            bshift += 1;
        }
        Self(inner)
    }

    pub const fn into_inner(self) -> [u8; 3] {
        self.0
    }

    pub const fn to_u8(self) -> u8 {
        let mut value: u8 = 0;
        let mut bit: usize = 7;
        let mut mshift: usize = 0;
        while bit <= 7 {
            if self.0[[0, 0, 0, 1, 1, 2, 2, 2][mshift]] & (0b1000_0000 >> (((mshift * 3) + 1) % 8))
                != 0
            {
                value |= 0b1 << bit
            }
            mshift += 1;
            bit = bit.wrapping_sub(1)
        }
        value
    }
}

impl Default for WireColor {
    fn default() -> Self {
        Self::new(0)
    }
}

impl From<u8> for WireColor {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

impl From<WireColor> for u8 {
    fn from(value: WireColor) -> Self {
        value.to_u8()
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
pub struct Pixel([WireColor; 3]);

impl PixelColor for Pixel {
    type Raw = RawU24;
}

impl Pixel {
    pub const fn new(g: u8, r: u8, b: u8) -> Self {
        Self([WireColor::new(g), WireColor::new(r), WireColor::new(b)])
    }

    pub const fn black() -> Self {
        Self::new(0, 0, 0)
    }

    pub const fn get_g(&self) -> u8 {
        self.0[0].to_u8()
    }

    pub const fn set_g(&mut self, g: u8) {
        self.0[0] = WireColor::new(g);
    }

    pub const fn get_r(&self) -> u8 {
        self.0[1].to_u8()
    }

    pub const fn set_r(&mut self, r: u8) {
        self.0[1] = WireColor::new(r);
    }

    pub const fn get_b(&self) -> u8 {
        self.0[2].to_u8()
    }

    pub const fn set_b(&mut self, b: u8) {
        self.0[2] = WireColor::new(b);
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
pub struct Ws2812Image<const W: usize, const H: usize> {
    pixels: [[Pixel; W]; H],
}

pub struct OutOfBoundsError;

#[cfg(feature = "defmt")]
impl defmt::Format for OutOfBoundsError {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "Pixel out of bounds")
    }
}

impl<const W: usize, const H: usize> Ws2812Image<W, H> {
    pub fn new() -> Self {
        assert!(W > 0);
        assert!(H > 0);
        Self {
            pixels: [[Pixel::new(0, 0, 0); W]; H],
        }
    }

    pub fn fill(&mut self, pixel: Pixel) {
        self.pixels.fill([pixel; W]);
    }

    pub fn clear(&mut self) {
        self.fill(Pixel::black());
    }

    pub fn set_pixels(&mut self, mut func: impl FnMut(usize, usize) -> Pixel) {
        for (y, r) in self.pixels.iter_mut().enumerate() {
            for (x, p) in r.iter_mut().enumerate() {
                *p = func(x, y)
            }
        }
    }

    pub fn set_pixel(&mut self, pixel: Pixel, x: usize, y: usize) -> Result<(), OutOfBoundsError> {
        let Some(pix) = self.pixels.get_mut(y).and_then(|p| p.get_mut(x)) else {
            return Err(OutOfBoundsError);
        };
        *pix = pixel;
        Ok(())
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<&Pixel> {
        self.pixels.get(y).and_then(|p| p.get(x))
    }

    pub fn get_pixel_mut(&mut self, x: usize, y: usize) -> Option<&mut Pixel> {
        self.pixels.get_mut(y).and_then(|p| p.get_mut(x))
    }

    pub fn get_row_mut(&mut self, row: usize) -> Option<&mut [Pixel; W]> {
        self.pixels.get_mut(row)
    }

    pub fn get_data_mut(&mut self) -> &mut [[Pixel; W]; H] {
        &mut self.pixels
    }

    fn as_ptr(&self) -> Option<NonNull<u8>> {
        self.pixels
            .first()
            .and_then(|p| p.first())
            .map(|p| ((&p.0[0]) as *const _) as *mut u8)
            .and_then(NonNull::new)
    }

    const TIMINGS_LEN: usize = size_of::<[[Pixel; W]; H]>();

    /// Returns row-major byte data of the image
    pub fn as_timings(&self) -> &[u8; Self::TIMINGS_LEN] {
        // Safety: Ws2812Image::new guarantees that pixels has len > 0, Self::TIMINGS_LEN
        // is guaranteed to be the same size as pixels
        unsafe { core::mem::transmute(&self.pixels) }
    }
}

impl<const W: usize, const H: usize> Dimensions for Ws2812Image<W, H> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        Rectangle::new(Point::zero(), Size::new(W as u32, H as u32))
    }
}

impl<const W: usize, const H: usize> DrawTarget for Ws2812Image<W, H> {
    type Color = Pixel;

    type Error = OutOfBoundsError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for pix in pixels {
            let x = pix.0.x.try_into();
            let y = pix.0.y.try_into();
            let (Ok(x), Ok(y)) = (x, y) else {
                continue;
            };
            let _ = self.set_pixel(pix.1, x, y);
        }
        Ok(())
    }
}
