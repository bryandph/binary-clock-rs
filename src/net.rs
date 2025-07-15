use core::net::{IpAddr, SocketAddr};
use cyw43::Control;
use cyw43::JoinOptions;
use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_net::udp::PacketMetadata;
use embassy_net::udp::UdpSocket;
use embassy_net::{Config, StackResources};
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_time::Duration;
use embassy_time::Timer;
use sntpc::get_time;
use sntpc::NtpContext;
use sntpc::NtpTimestampGenerator;
use static_cell::StaticCell;

use defmt::*;

const WIFI_NETWORK: &str = env!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
const NTP_SERVER: &str = env!("NTP_SERVER");

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

pub async fn setup_wifi(
    spawner: Spawner,
    spi: PioSpi<'static, PIO0, 0, DMA_CH0>,
    pwr: Output<'static>,
) -> (Control<'static>, embassy_net::Stack<'static>) {
    // let fw =
    //     include_bytes!("binary_clock\\embassy\\cyw43-firmware\\43439A0.bin");
    // let clm =
    //     include_bytes!("binary_clock\\embassy\\cyw43-firmware\\43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP235x --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP235x --base-address 0x10140000

    let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());
    let seed: u64 = 1748420917;

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    if !stack.is_link_up() {
        loop {
            match control
                .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
                .await
            {
                Ok(_) => break,
                Err(err) => {
                    info!("join failed with status={}", err.status);
                    Timer::after_millis(100).await;
                    continue;
                }
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    (control, stack)
}

#[derive(Copy, Clone, Default)]
struct Timestamp {
    duration: Duration,
}

impl NtpTimestampGenerator for Timestamp {
    fn init(&mut self) {
        self.duration = Duration::from_secs(0);
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        return self.duration.as_micros().try_into().unwrap();
    }
}

pub async fn get_ntptime(stack: embassy_net::Stack<'static>) -> i64 {
    stack.wait_config_up().await;
    // Create UDP socket
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(123).unwrap();

    let context = NtpContext::new(Timestamp::default());

    let ntp_addrs: heapless::Vec<embassy_net::IpAddress, 1> = stack
        .dns_query(NTP_SERVER, embassy_net::dns::DnsQueryType::A)
        .await
        .expect("Failed to resolve DNS");
    if ntp_addrs.is_empty() {
        defmt::panic!("Failed to resolve DNS");
    }
    let addr: IpAddr = ntp_addrs[0].into();
    let result = get_time(SocketAddr::from((addr, 123)), &socket, context).await;

    match result {
        Ok(time) => {
            info!("Time: {:?}", time);
            return time.offset();
        }
        Err(e) => {
            error!("Error getting time: {:?}", e);
            let timestamp: i64 = 100;
            return timestamp;
        }
    }
}
