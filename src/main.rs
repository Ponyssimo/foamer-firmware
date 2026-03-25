//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Connects to Wifi network and makes a web request to httpbin.org.

#![no_std]
#![no_main]
#![feature(int_from_ascii)]

use core::cell::OnceCell;
use core::net::{Ipv4Addr, SocketAddr};
use core::str::from_utf8;

use cyw43::JoinOptions;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::dns::{DnsQueryType, DnsSocket};
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config, IpAddress, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder, PioEncoderProgram};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, TrySendError};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, TimeoutError, Timer};
use embedded_nal_async::TcpConnect;
use reqwless::client::HttpClient;
// Uncomment these for TLS requests:
// use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::request::Method;
use serde::Deserialize;
use serde_json_core::from_slice;
use static_cell::StaticCell;
use strum::{EnumCount, EnumIter, VariantArray};
use {defmt_rtt as _, panic_probe as _};

use crate::profile::{Address, Function, Profile};
use crate::withrottle::{WiThrottleClient, WiThrottleError};

mod buf_reader;
mod profile;
mod withrottle;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

pub type CancellationSignal = Signal<CriticalSectionRawMutex, ()>;

const WIFI_NETWORK: &str = "parkerhouse"; // change to your network SSID
const WIFI_PASSWORD: &str = "password"; // change to your network password

async fn submit_request(request: WiThrottleRequest) {
    CANCELLATION_SIGNAL.signal(());
    if let Err(err) = WITHROTTLE_COMMAND_CHANNEL.try_send(request) {
        // Just in case they add variants later on, I want to handle them too...
        let request = match err {
            TrySendError::Full(request) => request,
        };
        error!(
            "Failed to send request {}... We're probably running out of space.",
            request
        );
    }
}

#[repr(u8)]
#[derive(Default, Format, VariantArray, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum ThrottlePosition {
    #[default]
    Idle,
    Notch1,
    Notch2,
    Notch3,
    Notch4,
    Notch5,
    Notch6,
    Notch7,
    Notch8,
}

#[repr(u8)]
#[derive(Default, Format, VariantArray, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum ReverserPosition {
    Reverse,
    #[default]
    Neutral,
    Forwards,
}

#[embassy_executor::task]
async fn read_throttle_encoder(mut encoder: PioEncoder<'static, PIO0, 1>) {
    let mut position = ThrottlePosition::default();
    loop {
        position = match encoder.read().await {
            Direction::Clockwise if position < ThrottlePosition::Notch8 => {
                ThrottlePosition::VARIANTS[position as usize + 1]
            }
            Direction::CounterClockwise if position > ThrottlePosition::Idle => {
                ThrottlePosition::VARIANTS[position as usize - 1]
            }
            _direction => {
                error!(
                    "Encoder is turning, but we are already in {}! Did we start desynced? Ignoring.",
                    position
                );
                position
            }
        };
        submit_request(WiThrottleRequest::SetThrottle(position)).await
    }
}

#[embassy_executor::task]
async fn read_reverser_encoder(mut encoder: PioEncoder<'static, PIO0, 2>) {
    let mut position = ReverserPosition::default();
    loop {
        position = match encoder.read().await {
            Direction::Clockwise if position < ReverserPosition::Forwards => {
                ReverserPosition::VARIANTS[position as usize + 1]
            }
            Direction::CounterClockwise if position > ReverserPosition::Reverse => {
                ReverserPosition::VARIANTS[position as usize - 1]
            }
            direction => {
                error!(
                    "Encoder is turning, but we are already in {}! Did we start desynced? Ignoring.",
                    position
                );
                position
            }
        };
        submit_request(WiThrottleRequest::SetReverser(position)).await
    }

    // let mut reverser: usize = 1;
    // loop {
    //     info!("Count: {}", count);
    //     match count.checked_add_signed(match encoder.read().await {
    //         Direction::Clockwise => 1,
    //         Direction::CounterClockwise => -1,
    //     }) {
    //         Some(value) => count = value,
    //         None => {
    //             error!("Encoder running backwards?");
    //         }
    //     }
    // }
}

#[embassy_executor::task]
async fn withrottle_heartbeat(rx: &'static Signal<CriticalSectionRawMutex, Instant>) -> ! {
    let mut deadline = rx.wait().await;
    loop {
        match embassy_time::with_deadline(deadline, rx.wait()).await {
            Ok(new_deadline) => {
                deadline = new_deadline;
            }
            Err(TimeoutError) => {
                submit_request(WiThrottleRequest::Heartbeat).await;
                // Don't do anything till we have a new one
                deadline = Instant::MAX;
            }
        }
    }
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[derive(Format)]
pub enum WiThrottleRequest {
    SetFunctionState(usize, bool),
    SetThrottle(ThrottlePosition),
    SetReverser(ReverserPosition),
    Heartbeat,
}

static WITHROTTLE_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, WiThrottleRequest, 16> =
    Channel::new();
static CANCELLATION_SIGNAL: CancellationSignal = CancellationSignal::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    // let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    // let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");
    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    let prg = PioEncoderProgram::new(&mut pio.common);
    let throttle_encoder = PioEncoder::new(&mut pio.common, pio.sm1, p.PIN_3, p.PIN_4, &prg);
    // This one is backwards!
    let reverser_encoder = PioEncoder::new(&mut pio.common, pio.sm2, p.PIN_5, p.PIN_6, &prg);
    defmt::unwrap!(spawner.spawn(read_throttle_encoder(throttle_encoder)));
    defmt::unwrap!(spawner.spawn(read_reverser_encoder(reverser_encoder)));

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    defmt::unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());
    // Use static IP configuration instead of DHCP
    //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    //});

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    defmt::unwrap!(spawner.spawn(net_task(runner)));

    while let Err(err) = control
        .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
        .await
    {
        info!("join failed with status={}", err.status);
    }

    info!("waiting for link...");
    stack.wait_link_up().await;

    info!("waiting for DHCP...");
    stack.wait_config_up().await;

    // And now we can use it!
    info!("Stack is up!");

    // And now we can use it!

    static LINE_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();
    let line_buffer = LINE_BUFFER.init([0; 4096]);
    static PROFILES: StaticCell<[Profile; 10]> = StaticCell::new();
    let profiles = PROFILES.init(core::array::from_fn(|_| Profile {
        address: Address::Long(0x1654),
        functions: [
            Some(Function {
                label: defmt::unwrap!("Bell".try_into()),
            }),
            Some(Function {
                label: defmt::unwrap!("Horn".try_into()),
            }),
            Some(Function {
                label: defmt::unwrap!("Amore".try_into()),
            }),
            None,
            None,
            None,
            None,
            None,
            None,
        ],
    }));

    static HEARTBEAT_DEADLINE: Signal<CriticalSectionRawMutex, Instant> = Signal::new();
    defmt::unwrap!(spawner.spawn(withrottle_heartbeat(&HEARTBEAT_DEADLINE)));

    loop {
        let mut rx_buffer = [0; 4096];
        // Uncomment these for TLS requests:
        // let mut tls_read_buffer = [0; 16640];
        // let mut tls_write_buffer = [0; 16640];

        let client_state = TcpClientState::<1, 4096, 4096>::new();
        let tcp_client = TcpClient::new(stack, &client_state);
        let dns_client = DnsSocket::new(stack);
        // Uncomment these for TLS requests:
        // let tls_config = TlsConfig::new(seed, &mut tls_read_buffer, &mut tls_write_buffer, TlsVerify::None);

        let withrottle_servers = dns_client
            .query("_withrottle._tcp.local", DnsQueryType::A)
            .await
            .unwrap_or_else(|_| {
                defmt::unwrap!(heapless::Vec::from_slice(&[IpAddress::Ipv4(
                    Ipv4Addr::new(192, 168, 32, 120),
                )]))
            });
        info!("Found withrottle servers: {}", withrottle_servers);
        let withrottle_socket = match withrottle_servers.first() {
            Some(&address) => match tcp_client
                .connect(SocketAddr::new(address.into(), 12090))
                .await
            {
                Ok(socket) => {
                    info!("Connected to withrottle sever! {}", address);
                    socket
                }
                Err(err) => {
                    error!(
                        "Couldn't connect to withrottle server at {}: {}",
                        address, err
                    );
                    continue;
                }
            },
            _ => continue,
        };
        let hardware_address = stack.hardware_address();
        let hardware_address_bytes = hardware_address.as_bytes();
        let mut id = [0u8; 32];
        let id = base16ct::lower::encode(&hardware_address_bytes, &mut id).unwrap();

        let mut withrottle_client = match WiThrottleClient::new(
            withrottle_socket,
            id,
            &profiles[0],
            line_buffer,
            &HEARTBEAT_DEADLINE,
        )
        .await
        {
            Ok(client) => client,
            Err(err) => {
                error!("Failed to connect to withrottle server: {}", err);
                continue;
            }
        };

        loop {
            while let Ok(message) = WITHROTTLE_COMMAND_CHANNEL.try_receive() {
                info!("Got a message from the command channel: {}", message);
                let result = async {
                    match message {
                        WiThrottleRequest::SetFunctionState(function_id, state) => {
                            &withrottle_client;
                            defmt::info!("Set the state with the withrottle client");
                        }
                        WiThrottleRequest::SetReverser(direction) => {
                            withrottle_client.set_direction(direction).await?;
                        }
                        WiThrottleRequest::SetThrottle(position) => {
                            withrottle_client.set_speed((position as u8) * 3).await?;
                        }
                        WiThrottleRequest::Heartbeat => {
                            withrottle_client.heartbeat().await?;
                        }
                    }
                    Ok::<_, WiThrottleError>(())
                }
                .await;
                match result {
                    Ok(()) => {}
                    Err(err) => {
                        error!("Failed to handle request {}! {}", message, err);
                    }
                }
            }
            match withrottle_client
                .handle_line(Some(&CANCELLATION_SIGNAL))
                .await
            {
                Ok(()) | Err(WiThrottleError::Cancelled) => {}
                Err(err) => {
                    error!("WiThrottle client failed: {}", err);
                    break;
                }
            }
        }
        info!("Guess we got disconnected... Looping over.");
    }
}
