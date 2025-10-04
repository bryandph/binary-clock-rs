#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

mod clock;
mod net;
mod rtg;

use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;

use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{PIO0, PIO1};
use embassy_rp::pio::{InterruptHandler, Pio};
use rtg::set_time;
use smart_leds::RGB8;

use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_time::{Duration, Ticker, Timer};

use {defmt_rtt as _, panic_probe as _};

// Program metadata for `picotool info`.
// This isn't needed, but it's recommended to have these minimal entries.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Blinky Example"),
    embassy_rp::binary_info::rp_program_description!(
        c"This example tests the RP Pico 2 W's onboard LED, connected to GPIO 0 of the cyw43 \
        (WiFi chip) via PIO 0 over the SPI bus."
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Init wifi hardware
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut wifipio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut wifipio.common,
        wifipio.sm0,
        RM2_CLOCK_DIVIDER,
        wifipio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );
    let (mut control, stack) = net::setup_wifi(spawner, spi, pwr).await;

    let cur_time: i64 = net::get_ntptime(stack).await;
    set_time(cur_time);

    control.leave().await;

    let mut ledpio = Pio::new(p.PIO1, Irqs);

    let ws2812_prg = PioWs2812Program::new(&mut ledpio.common);
    let mut ws2812 = PioWs2812::new(
        &mut ledpio.common,
        ledpio.sm0,
        p.DMA_CH1,
        p.PIN_6,
        &ws2812_prg,
    );

    // spawner.spawn(clock::displayloop(ws2812, control)).unwrap();
    let mut data = [RGB8::default(); clock::NUM_LEDS];

    let delay = Duration::from_millis(10);
    // Loop forever making RGB values and pushing them out to the WS2812.
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        info!("led on!");
        control.gpio_set(0, true).await;
        let now = rtg::now();
        clock::dttobcd(&mut data, now, 16, 10);
        clock::brightness(&mut data, 250);
        ws2812.write(&data).await;

        Timer::after(delay).await;
        info!("led off!");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;

        ticker.next().await;
    }
}
