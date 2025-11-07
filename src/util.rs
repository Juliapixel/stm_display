use core::ops::{Deref, DerefMut};

use embedded_graphics::{
    prelude::{Dimensions, DrawTarget, Size},
    primitives::Rectangle,
};

pub struct Transposed<D: DrawTarget>(pub D);

impl<D: DrawTarget> Deref for Transposed<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: DrawTarget> DerefMut for Transposed<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<D: DrawTarget> Dimensions for Transposed<D> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        let b = self.0.bounding_box();
        Rectangle::new(b.top_left, Size::new(b.size.height, b.size.width))
    }
}

impl<D: DrawTarget> DrawTarget for Transposed<D> {
    type Color = D::Color;

    type Error = D::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        self.0.draw_iter(pixels.into_iter().map(|mut p| {
            (p.0.x, p.0.y) = (p.0.y, p.0.x);
            p
        }))
    }
}

pub struct Staggered<D: DrawTarget>(pub D);

impl<D: DrawTarget> Deref for Staggered<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: DrawTarget> DerefMut for Staggered<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<D: DrawTarget> Dimensions for Staggered<D> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        self.0.bounding_box()
    }
}

impl<D: DrawTarget> DrawTarget for Staggered<D> {
    type Color = D::Color;

    type Error = D::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let width = self.bounding_box().size.width;
        self.0.draw_iter(pixels.into_iter().map(|mut p| {
            if p.0.y % 2 == 1 {
                p.0.x = ((p.0.x - (p.0.x * 2)) + width as i32 - 1).abs();
            }

            p
        }))
    }
}
