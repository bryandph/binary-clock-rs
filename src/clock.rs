use crate::{rtg, Irqs};
use chrono::{DateTime, Datelike, TimeZone, Timelike};
use chrono_tz::Tz;
use chrono_tz::US::Central;
use cyw43::Control;
use defmt::*;
use embassy_rp::peripherals::PIO1;
use embassy_rp::pio::Pio;
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_time::{Duration, Ticker, Timer};
use heapless::{String, Vec};
use smart_leds::colors::{BLACK, WHITE};
use smart_leds::RGB8;

use {defmt_rtt as _, panic_probe as _};

pub const NUM_LEDS: usize = 160;

/// Input a value 0 to 255 to get a color value
/// The colours are a transition r - g - b - back to r.
pub fn _wheel(mut wheel_pos: u8) -> RGB8 {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        return (255 - wheel_pos * 3, 0, wheel_pos * 3).into();
    }
    if wheel_pos < 170 {
        wheel_pos -= 85;
        return (0, wheel_pos * 3, 255 - wheel_pos * 3).into();
    }
    wheel_pos -= 170;
    (wheel_pos * 3, 255 - wheel_pos * 3, 0).into()
}

pub fn brightness(data: &mut [smart_leds::RGB<u8>; NUM_LEDS], level: u8) {
    for pixel in data.iter_mut() {
        pixel.r = pixel.r.saturating_sub(level);
        pixel.g = pixel.g.saturating_sub(level);
        pixel.b = pixel.b.saturating_sub(level);
    }
}

fn pad2(num: u32) -> String<2> {
    let mut s = String::<2>::new();
    let tens = num / 10;
    let ones = num % 10;
    s.push((b'0' + tens as u8) as char).unwrap();
    s.push((b'0' + ones as u8) as char).unwrap();
    s
}

fn pad4(num: u32) -> String<4> {
    let mut s = String::<4>::new();
    let thousands = num / 1000;
    let hundreds = (num / 100) % 10;
    let tens = (num / 10) % 10;
    let ones = num % 10;
    s.push((b'0' + thousands as u8) as char).unwrap();
    s.push((b'0' + hundreds as u8) as char).unwrap();
    s.push((b'0' + tens as u8) as char).unwrap();
    s.push((b'0' + ones as u8) as char).unwrap();
    s
}

// returns the reversed 4-bit binary string for a given digit.
fn digit_to_rev_bin(digit: u32) -> String<4> {
    let mut s = String::<4>::new();
    for bit in 0..4 {
        s.push(if (digit >> bit) & 1 == 1 { '1' } else { '0' })
            .unwrap();
    }
    s
}

// draws an array of digits starting at a given vertical offset.
fn draw_digits(
    data: &mut [smart_leds::RGB<u8>; NUM_LEDS],
    digits: &[u32],
    width: usize,
    y_offset: usize,
) {
    for (d, &digit) in digits.iter().enumerate() {
        let x = width - d - 1;
        let rev_bin = digit_to_rev_bin(digit);
        for (y, ch) in rev_bin.chars().enumerate() {
            let idx = x + (y_offset + y) * width;
            if idx < data.len() {
                data[idx] = if ch == '1' { WHITE } else { BLACK };
            }
        }
    }
}

// Display the current time as binery encoded decimal pixels on a 2d array
pub fn dttobcd(
    data: &mut [smart_leds::RGB<u8>; NUM_LEDS],
    timestamp: u64,
    width: usize,
    _height: usize,
) {
    // Clear the entire LED display.
    for pixel in data.iter_mut() {
        *pixel = BLACK;
    }

    let now_utc = DateTime::from_timestamp_micros(timestamp.try_into().unwrap()).unwrap();
    let now: DateTime<Tz> = Central.from_utc_datetime(&now_utc.naive_utc());

    let hour = now.hour();
    let minute = now.minute();
    let second = now.second();

    let hour_str = pad2(hour);
    let minute_str = pad2(minute);
    let second_str = pad2(second);

    let time_digits: Vec<u32, 6> = hour_str
        .chars()
        .chain(minute_str.chars())
        .chain(second_str.chars())
        .map(|c| c.to_digit(10).unwrap())
        .collect();
    info!(
        "time: {} {} {}",
        hour_str.as_str(),
        minute_str.as_str(),
        second_str.as_str()
    );

    // Draw time digits with no vertical offset.
    draw_digits(data, &time_digits, width, 0);

    // Get date components.
    // Note: day calculation remains as earlier.
    let day = now.day() + 1 as u32;
    let month = now.month() as u32;
    let year = now.year().unsigned_abs();

    let day_str = pad2(day);
    let month_str = pad2(month);
    let year_str = pad4(year);

    let date_digits: Vec<u32, 8> = day_str
        .chars()
        .chain(month_str.chars())
        .chain(year_str.chars())
        .map(|c| c.to_digit(10).unwrap())
        .collect();
    info!(
        "date: {}-{}-{}",
        day_str.as_str(),
        month_str.as_str(),
        year_str.as_str()
    );

    // Draw date digits with a vertical offset (5 rows).
    draw_digits(data, &date_digits, width, 5);
}

#[embassy_executor::task]
pub async fn displayloop(
    mut ws2812: PioWs2812<'static, PIO1, 0, NUM_LEDS>,
    mut control: Control<'static>,
) {
    let mut data = [RGB8::default(); NUM_LEDS];

    let delay = Duration::from_millis(10);
    // Loop forever making RGB values and pushing them out to the WS2812.
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        info!("led on!");
        control.gpio_set(0, true).await;
        let now = rtg::now();
        dttobcd(&mut data, now, 16, 10);
        brightness(&mut data, 250);
        ws2812.write(&data).await;

        Timer::after(delay).await;
        info!("led off!");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;

        ticker.next().await;
    }
}
