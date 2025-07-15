# RP2350 Binary Clock

This project implements a binary clock on an [RP2350 (Raspberry Pi Pico 2 W) Wifi-enabled microcontroller](https://www.mouser.com/ProductDetail/Raspberry-Pi/SC1633?qs=3vio67wFuYrX4U8ue%2Fh1eA%3D%3D). It is written using the [`embassy-rs`](https://github.com/embassy-rs/embassy) embeded framework and targets a [Waveshare 16x10 LED Matrix Panel](https://www.waveshare.com/pico-rgb-led.htm) over serial. Support for WS2812B LEDs is provided by [`embassy-rp` ](https://github.com/embassy-rs/embassy/blob/main/embassy-rp/src/pio_programs/ws2812.rs) which runs communication with the LEDs in a [Pico PIO state machine](https://www.raspberrypi.com/news/what-is-pio/). Accurate time is provided by an NTP client running on the controller.

## Motivation
I wanted to do some embeded Rust and play with Embassy. This is a learning project that shows how to run async tasks, passing data between processes, and networking in Embassy.

## Development Notes

### RP2350 Compatibility

The RP2350 support in Embassy is relatively new but functions similarly to the RP2040. One notable difference is the Real Time Clock implementation:

- **RP2040**: Has dedicated RTC peripheral
- **RP2350**: Uses Always-On Timer instead of RTC

Since `embassy-rp` doesn't yet have RTC implementation for RP2350, this project includes a custom interface to read/write time from the appropriate registers.

## Known Limitations & Future Improvements

- [ ] **Periodic NTP Updates**: Currently only syncs time on boot
- [ ] **Error Handling**: Improve error reporting and recovery mechanisms
- [ ] **Display Modes**: Add different time display formats and timezones besides UTC
- [ ] **Configuration**: Runtime WiFi and NTP configuration
- [ ] **Power Management**: Implement sleep modes for battery operation
- [ ] **UX**: Use the LED matrix more extensivly


## Building

Ensure the following environment variables are set before attempting to build with cargo:
* `WIFI_NETWORK`
* `WIFI_PASSWORD`
* `NTP_SERVER`

You will need to flash the WiFi firmware (download it from the [Embassy repo](https://github.com/embassy-rs/embassy)) ahead of time using the following commands:
```bash
probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000

```
You can also configure the program to include the firmware at at compile time by adding the following lines to `net.rs` and commenting out the eixsting `fw` and `cli` declerations:
```rust
let fw = include_bytes!("path-to/embassy/cyw43-firmware/43439A0.bin");
let clm = include_bytes!("path-to/embassy/cyw43-firmware/43439A0_clm.bin");
```

This project uses `cargo-embed` from `probe-rs-tools` and requires `flip-link` to be avaliable. Ensure the `thumbv8m.main-none-eabihf` Rust target is installed and then use `cargo embed --release` to run it on an attached RP2035 via a Pico Probe debugger. The program is configured to send RTT data over an attached SWD probe for debugging purposes.

A Nix [devenv](https://github.com/cachix/devenv) is provided to get a working environment quickly, but most development took place on Windows 11, and there are no OS dependencies for the project beside what is required by `probe-rs`.