#![no_std]
#![no_main]
#![feature(generic_const_exprs)]

#![deny(clippy::missing_safety_doc)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::multiple_unsafe_ops_per_block)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_unsafe)]

mod fmt;
mod util;
mod ws2812;

use crate::{
    fmt::{debug, info},
    util::{Staggered, Transposed},
    ws2812::{Pixel, Ws2812Image},
};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver},
};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    Drawable,
    mono_font::MonoTextStyle,
    prelude::Point,
    text::{Text, renderer::TextRenderer},
};
#[cfg(not(feature = "defmt"))]
use panic_halt as _;
use static_cell::StaticCell;

#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_stm32::{
    Config,
    gpio::{Level, Output, Speed},
    mode::Async,
    rcc::{Config as RccConfig, Hse, Pll},
    spi::Spi,
    time::Hertz,
};

static CHAN: StaticCell<Channel<NoopRawMutex, [u8; 8 * 32 * 3 * 3], 2>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut rcc = RccConfig::default();
    rcc.hse = Some(Hse {
        freq: Hertz(25_000_000),
        mode: embassy_stm32::rcc::HseMode::Oscillator,
    });
    rcc.sys = embassy_stm32::rcc::Sysclk::PLL1_P;
    rcc.pll_src = embassy_stm32::rcc::PllSource::HSE;
    rcc.pll = Some(Pll {
        prediv: embassy_stm32::rcc::PllPreDiv::DIV25,
        mul: embassy_stm32::rcc::PllMul::MUL199,
        divp: Some(embassy_stm32::rcc::PllPDiv::DIV2),
        divq: None,
        divr: None,
    });

    rcc.apb1_pre = embassy_stm32::rcc::APBPrescaler::DIV2;
    let mut config = Config::default();
    config.rcc = rcc;

    let p = embassy_stm32::init(config);
    info!("started");

    let mut spi_config = embassy_stm32::spi::Config::default();

    spi_config.mode = embassy_stm32::spi::MODE_2;
    spi_config.bit_order = embassy_stm32::spi::BitOrder::MsbFirst;
    spi_config.frequency = embassy_stm32::time::Hertz(2500000);
    spi_config.gpio_speed = Speed::High;

    let spi = Spi::new_txonly_nosck(p.SPI1, p.PB5, p.DMA2_CH2, spi_config);
    let led = Output::new(p.PC13, Level::High, Speed::Low);
    let chan = CHAN.init(Channel::<NoopRawMutex, _, 2>::new());
    spawner.spawn(send_img(spi, chan.receiver(), led)).unwrap();

    let mut img = Transposed(Staggered(Ws2812Image::<8, 32>::new()));

    let mono_text_style = MonoTextStyle::new(
        &embedded_graphics::mono_font::ascii::FONT_4X6,
        Pixel::new(8, 8, 8),
    );

    let mut text = Text::new("https://juliapixel.com", Point::new(0, 5), mono_text_style);

    let size = text.character_style.measure_string(
        text.text,
        Point::zero(),
        embedded_graphics::text::Baseline::Bottom,
    );
    text.position.x = size.bounding_box.size.width as i32 + 32;

    loop {
        let start = Instant::now();
        img.fill(Pixel::default());

        text.character_style.text_color = Some(Pixel::new(2, 16, 4));
        text.position.x -= 1;
        if text.position.x <= -(size.bounding_box.size.width as i32) {
            text.position.x = size.bounding_box.size.width as i32 + 32;
        }

        let _ = text.draw(&mut img);

        let timings = img.as_timings();
        chan.send(*timings).await;
        Timer::at(start.saturating_add(Duration::from_millis(33))).await;
    }
}

#[embassy_executor::task]
async fn send_img(
    mut spi: Spi<'static, Async>,
    chan: Receiver<'static, NoopRawMutex, [u8; 8 * 32 * 3 * 3], 2>,
    mut pin: Output<'static>,
) {
    let mut times = heapless::Deque::<Duration, 32>::new();
    for i in 0.. {
        pin.set_high();
        let start = Instant::now();
        let timings = chan.receive().await;
        pin.set_low();
        spi.write(&[0u8; 150]).await.unwrap();
        spi.write(&timings).await.unwrap();
        let elapsed = start.elapsed();
        if times.is_full() {
            times.pop_front();
        }
        let _ = times.push_back(elapsed);
        if i % 100 == 0 {
            let avg: u64 = times.iter().map(|t| t.as_micros()).sum();
            debug!("Took {}us on average", avg / times.len() as u64);
        }
    }
}
