#![no_std]
#![feature(type_alias_impl_trait)]

use embedded_graphics::pixelcolor::Rgb888;

#[cfg(feature = "esp32s3")]
pub mod framebuffer;
pub mod gpio;
#[cfg(feature = "esp32s3")]
pub mod lcd_cam;

pub type Color = Rgb888;
