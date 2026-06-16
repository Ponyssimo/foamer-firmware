//! This example uses the RP Pico W board Wifi chip (cyw43).
//! Connects to Wifi network and makes a web request to httpbin.org.

#![no_std]
#![no_main]

use core::cell::RefCell;
use core::net::SocketAddr;

use crate::rotary_switch::RotarySwitch;
use crate::triple_switch::TripleSwitch;
use critical_section::Mutex;
use cyw43::JoinOptions;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::join::join_array;
use embassy_net::dns::{DnsQueryType, DnsSocket};
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config as NetConfig, StackResources};
use embassy_rp::Peri;
use embassy_rp::adc::{self, Adc};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::dma;
use embassy_rp::gpio::{Input, Level, Output, Pin, Pull};
use embassy_rp::peripherals::{DMA_CH0, PIO0, USB};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder, PioEncoderProgram};
use embassy_rp::pwm::{self, Pwm};
use embassy_rp::usb::{self, Driver};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, TrySendError};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, TimeoutError, Timer};
use embassy_usb::class::{
    // cdc_acm::{CdcAcmClass, State},
    web_usb::{Config as WebUsbConfig, State, Url, WebUsb},
};
use embassy_usb::{
    UsbDevice,
    msos::{self, windows_version},
};
use embedded_nal_async::TcpConnect;
use foamer_types::{
    BRAKE_START_INDEX, BrakeState, Config, HORN_INDEX, TRIPLE_SWITCH_FUNCTION_COUNT,
    TRIPLE_SWITCH_START_INDEX, TRIPLE_SWITCHES, TripleSwitchState, USER_BUTTONS,
    WiThrottleDiscovery,
};
use panic_probe as _;
use static_cell::StaticCell;
use strum::VariantArray;

use crate::flash::FlashCommand;
use crate::profile_usb::ProfileUsbEndpoints;
use crate::withrottle::{ProfileWrapper, WiThrottleClient, WiThrottleError};

mod buf_reader;
mod flash;
mod profile_usb;
mod rotary_switch;
mod triple_switch;
mod withrottle;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    ADC_IRQ_FIFO => adc::InterruptHandler;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

pub type CancellationSignal = Signal<CriticalSectionRawMutex, ()>;

static STATUS_LIGHT: Mutex<RefCell<Option<Output<'static>>>> = Mutex::new(RefCell::new(None));

#[cortex_m_rt::exception]
unsafe fn HardFault(_ef: &cortex_m_rt::ExceptionFrame) -> ! {
    loop {
        critical_section::with(|cs| {
            if let Some(output) = STATUS_LIGHT.borrow_ref_mut(cs).as_mut() {
                output.toggle();
            }
        });
        cortex_m::asm::delay(15_000_000);
    }
}

pub fn submit_request_sync(request: WiThrottleRequest) {
    CANCELLATION_SIGNAL.signal(());
    if let Err(err) = WITHROTTLE_COMMAND_CHANNEL.try_send(request) {
        // Just in case they add variants later on, I want to handle them too...
        #[allow(clippy::infallible_destructuring_match)]
        let request = match err {
            TrySendError::Full(request) => request,
        };
        error!(
            "Failed to send request {}... We're probably running out of space.",
            request
        );
    }
}

pub async fn submit_request(request: WiThrottleRequest) {
    submit_request_sync(request)
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

struct TripleSwitchInputs<'a> {
    switches: [TripleSwitch<'a>; TRIPLE_SWITCHES],
}

impl<'a> TripleSwitchInputs<'a> {
    fn new(
        pin_0: Peri<'a, impl Pin>,
        pin_1: Peri<'a, impl Pin>,
        pin_2: Peri<'a, impl Pin>,
    ) -> Self {
        Self {
            switches: [
                TripleSwitch::new(pin_0),
                TripleSwitch::new(pin_1),
                TripleSwitch::new(pin_2),
            ],
        }
    }

    fn read_all(&mut self) -> impl Future<Output = [TripleSwitchState; TRIPLE_SWITCHES]> {
        join_array(self.switches.each_mut().map(|switch| switch.read()))
    }
}

struct UserInputs<'a> {
    user: [Input<'a>; USER_BUTTONS],
    profile: RotarySwitch<'a>,
}

#[embassy_executor::task]
async fn read_function_buttons(mut user_inputs: UserInputs<'static>) {
    let mut old_values = None::<[bool; USER_BUTTONS]>;
    let mut old_profile = None::<u8>;
    loop {
        let new_values = user_inputs.user.each_ref().map(|input| input.is_high());
        for (index, (_old, new)) in
            core::iter::zip(old_values.transpose().into_iter(), new_values.into_iter())
                .enumerate()
                .filter(|(_index, (old, new))| *old != Some(*new))
        {
            defmt::info!("State of {} changed to {}", index, new);
            submit_request(WiThrottleRequest::SetFunctionState(index, new)).await;
        }
        old_values = Some(new_values);

        let new_profile = user_inputs.profile.read();
        if new_profile != old_profile
            && let Some(new_profile) = new_profile
        {
            submit_request(WiThrottleRequest::SetProfile(new_profile as usize)).await;
            old_profile = Some(new_profile);
        }

        embassy_futures::select::select(
            embassy_futures::select::select_array(
                user_inputs
                    .profile
                    .pins
                    .each_mut()
                    .map(|input| input.wait_for_any_edge()),
            ),
            embassy_futures::select::select_array(
                user_inputs
                    .user
                    .each_mut()
                    .map(|input| input.wait_for_any_edge()),
            ),
        )
        .await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn read_throttle_encoder(mut encoder: PioEncoder<'static, PIO0, 1>) {
    let mut position = ThrottlePosition::default();
    loop {
        position = match encoder.read().await {
            Direction::CounterClockwise if position < ThrottlePosition::Notch8 => {
                ThrottlePosition::VARIANTS[position as usize + 1]
            }
            Direction::Clockwise if position > ThrottlePosition::Idle => {
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
                    "Encoder is turning in direction {}, but we are already in {}! Did we start desynced? Ignoring.",
                    matches!(direction, Direction::Clockwise),
                    position
                );
                position
            }
        };
        submit_request(WiThrottleRequest::SetReverser(position)).await
    }
}

trait OptionArrayExt<const N: usize> {
    type Inner;

    fn transpose(self) -> [Option<Self::Inner>; N];
}
impl<T, const N: usize> OptionArrayExt<N> for Option<[T; N]> {
    type Inner = T;

    fn transpose(self) -> [Option<Self::Inner>; N] {
        self.map(|array| array.map(Some))
            .unwrap_or_else(|| [const { None }; N])
    }
}

#[embassy_executor::task]
async fn read_triple_switches(mut triple_switch_inputs: TripleSwitchInputs<'static>) {
    let mut old = None::<[TripleSwitchState; 3]>;
    loop {
        let new = triple_switch_inputs.read_all().await;
        for (index, (old, new)) in core::iter::zip(
            // Option<[T; N]> -> [Option<T>; N]
            old.transpose().into_iter(),
            new.into_iter(),
        )
        .enumerate()
        .filter(|(_, (old, new))| *old != Some(*new))
        {
            submit_request(WiThrottleRequest::SetFunctionState(
                TRIPLE_SWITCH_START_INDEX + (index * TRIPLE_SWITCH_FUNCTION_COUNT) + (new as usize),
                true,
            ))
            .await;

            if let Some(old) = old {
                submit_request(WiThrottleRequest::SetFunctionState(
                    TRIPLE_SWITCH_START_INDEX
                        + (index * TRIPLE_SWITCH_FUNCTION_COUNT)
                        + (old as usize),
                    false,
                ))
                .await;
            }
        }
        old = Some(new);

        Timer::after(Duration::from_millis(50)).await;
    }
}

#[embassy_executor::task]
async fn read_potentiometers(
    mut adc: Adc<'static, embassy_rp::adc::Async>,
    mut brake: adc::Channel<'static>,
    mut horn: adc::Channel<'static>,
    mut pot_drv: Output<'static>,
) {
    let mut horn_released = None::<u16>;
    let mut last_brake_state = BrakeState::Released;
    let mut last_horn_state = false;
    let mut brake_released = None::<u16>;
    loop {
        pot_drv.set_high();
        // Make sure the electricity is flowing!
        Timer::after(Duration::from_micros(100)).await;
        let brake = defmt::unwrap!(adc.read(&mut brake).await, "Brake conversion error");
        let horn = defmt::unwrap!(adc.read(&mut horn).await, "Horn conversion error");
        pot_drv.set_low();

        match horn_released {
            Some(horn_released) => {
                // Horn apply causes horn value to DECREASE
                let horn_state = horn < horn_released && ((horn_released - horn) > 10);
                if horn_state != last_horn_state {
                    defmt::warn!("Horn state now: {}", horn_state);
                    submit_request(WiThrottleRequest::SetFunctionState(HORN_INDEX, horn_state))
                        .await;
                    last_horn_state = horn_state;
                }
            }
            None => {
                horn_released = Some(horn);
            }
        }

        match brake_released {
            Some(brake_released) => {
                let min = brake_released;
                let max = brake_released + 2000u16;
                let step = (max - min) / BrakeState::VARIANTS.len() as u16;
                let mut brake_state = BrakeState::Released;
                for (index, state) in BrakeState::VARIANTS.iter().copied().enumerate() {
                    let index = index as u16;
                    if brake > min + (step * index) {
                        brake_state = state;
                    }
                }
                if brake_state != last_brake_state {
                    for (index, state) in BrakeState::VARIANTS.iter().copied().enumerate() {
                        let index = index as u16;
                        submit_request(WiThrottleRequest::SetFunctionState(
                            BRAKE_START_INDEX + index as usize,
                            brake_state == state,
                        ))
                        .await;
                    }
                }
                last_brake_state = brake_state;
            }
            None => {
                brake_released = Some(brake);
            }
        }

        // Brake stuff

        Timer::after(Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task]
async fn withrottle_heartbeat(rx: &'static Signal<CriticalSectionRawMutex, Duration>) -> ! {
    let mut interval = rx.wait().await;
    let mut deadline = Instant::now().checked_add(interval).unwrap_or(Instant::MAX);
    loop {
        match embassy_time::with_deadline(deadline, rx.wait()).await {
            Ok(new_interval) => {
                defmt::info!("Got new heartbeat interval: {}", new_interval);
                interval = new_interval;
                deadline = Instant::now().checked_add(interval).unwrap_or(Instant::MAX);
            }
            Err(TimeoutError) => {
                defmt::debug!("Heartbeat time!");
                submit_request(WiThrottleRequest::Heartbeat).await;
                // Don't do anything till we have a new one
                deadline = Instant::now().checked_add(interval).unwrap_or(Instant::MAX);
            }
        }
    }
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
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
    SetProfile(usize),
    Heartbeat,
    Disconnect,
}

#[embassy_executor::task]
async fn usb_task(mut usb: UsbDevice<'static, Driver<'static, USB>>) -> ! {
    usb.run().await
}

static WITHROTTLE_COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, WiThrottleRequest, 16> =
    Channel::new();
static CANCELLATION_SIGNAL: CancellationSignal = CancellationSignal::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let mut p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let status_light = Output::new(p.PIN_14, Level::High);
    critical_section::with(|cs| {
        *STATUS_LIGHT.borrow_ref_mut(cs) = Some(status_light);
    });
    let mut connected_light = Output::new(p.PIN_15.reborrow(), Level::Low);

    let mut flash =
        embassy_rp::flash::Flash::<_, flash::Blocking, { flash::FLASH_SIZE }>::new_blocking(
            p.FLASH,
        );
    static CONFIG: StaticCell<Mutex<RefCell<Config>>> = StaticCell::new();
    defmt::info!("Going to grab the config!");
    let config = CONFIG.init(Mutex::new(RefCell::new(
        match flash::read_config(&mut flash) {
            Ok(config) => config,
            Err(err) => {
                defmt::error!("Invalid config! Giving a default instead... {}", err);
                Default::default()
            }
        },
    )));

    // Start wifi stuff:

    let fw = cyw43::aligned_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = cyw43::aligned_bytes!("../cyw43-firmware/43439A0_clm.bin");
    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    // let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    // let fw = unsafe { &*(fw as *const [u8] as *const cyw43::Aligned<cyw43::A4, [u8]>) };
    // let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };
    let nvram = cyw43::aligned_bytes!("../cyw43-firmware/nvram_rp2040.bin");

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
        embassy_rp::dma::Channel::new(p.DMA_CH0, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(defmt::unwrap!(cyw43_task(runner)));

    // USB shit

    // Create the driver, from the HAL.
    let driver = Driver::new(p.USB, Irqs);

    static HARDWARE_ID: StaticCell<[u8; 32]> = StaticCell::new();
    let hardware_id =
        base16ct::lower::encode_str(&control.address().await, HARDWARE_ID.init([0; _])).unwrap();

    let usb_config = {
        let mut config = embassy_usb::Config::new(0x0403, 0x698F);
        config.manufacturer = Some("Mary Strodl");
        config.product = Some("Foamer");
        config.serial_number = Some(hardware_id);
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config
    };

    static WEBUSB_STATE: StaticCell<State> = StaticCell::new();
    let state = WEBUSB_STATE.init(State::new());
    static WEBUSB_CONFIG: StaticCell<WebUsbConfig> = StaticCell::new();
    let webusb_config = WEBUSB_CONFIG.init(WebUsbConfig {
        max_packet_size: 64,
        vendor_code: 1,
        // If defined, shows a landing page which the device manufacturer would like the user to visit in order to control their device. Suggest the user to navigate to this URL when the device is connected.
        landing_url: Some(Url::new("https://foamer.mstrodl.com")),
    });

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        // This is a randomly generated GUID to allow clients on Windows to find our device
        const DEVICE_INTERFACE_GUIDS: &[&str] = &["{0DFAA759-5E78-4411-9463-A3158433C0CD}"];

        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let mut builder = embassy_usb::Builder::new(
            driver,
            usb_config,
            CONFIG_DESCRIPTOR.init([0; _]),
            BOS_DESCRIPTOR.init([0; _]),
            MSOS_DESCRIPTOR.init([0; _]),
            CONTROL_BUF.init([0; _]),
        );

        builder.msos_descriptor(windows_version::WIN8_1, 0);
        builder.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
        builder.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
            "DeviceInterfaceGUIDs",
            msos::PropertyData::RegMultiSz(DEVICE_INTERFACE_GUIDS),
        ));

        WebUsb::configure(&mut builder, state, webusb_config);

        builder
    };

    let profile_usb_endpoints = ProfileUsbEndpoints::new(webusb_config, &mut builder);

    // Build the builder.
    let usb = builder.build();

    // Run the USB device.
    spawner.spawn(unwrap!(usb_task(usb)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let net_config = NetConfig::dhcpv4(Default::default());
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
        net_config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    let wifi_config = config.get_mut().get_mut().wifi_config.clone();

    static FLASH_CHANNEL: StaticCell<Channel<CriticalSectionRawMutex, FlashCommand, 1>> =
        StaticCell::new();
    let flash_channel = FLASH_CHANNEL.init(Channel::new());
    spawner.spawn(defmt::unwrap!(crate::flash::flash_task(
        flash,
        config,
        flash_channel.receiver()
    )));
    spawner.spawn(defmt::unwrap!(crate::profile_usb::usb_task(
        profile_usb_endpoints,
        config,
        flash_channel.sender(),
    )));
    let user1 = Input::new(p.PIN_21, Pull::Down);
    if user1.is_high() {
        let mut config = pwm::Config::default();
        config.top = 32_768;
        config.compare_b = 8;
        core::mem::drop(connected_light);
        let mut pwm = Pwm::new_output_b(p.PWM_SLICE7, p.PIN_15, config.clone());
        let mut dir = false;
        loop {
            loop {
                let result = if dir {
                    config.compare_b.checked_shr(1)
                } else {
                    config.compare_b.checked_shl(1)
                };
                match result {
                    Some(0) | None => {
                        dir = !dir;
                    }
                    Some(value) => {
                        config.compare_b = value;
                        break;
                    }
                }
            }
            pwm.set_config(&config);
            Timer::after(Duration::from_millis(200)).await;
        }
    }

    let prg = PioEncoderProgram::new(&mut pio.common);
    let throttle_encoder = PioEncoder::new(&mut pio.common, pio.sm1, p.PIN_3, p.PIN_4, &prg);
    // This one is backwards!
    let reverser_encoder = PioEncoder::new(&mut pio.common, pio.sm2, p.PIN_5, p.PIN_6, &prg);
    spawner.spawn(defmt::unwrap!(read_throttle_encoder(throttle_encoder)));
    spawner.spawn(defmt::unwrap!(read_reverser_encoder(reverser_encoder)));

    let adc = Adc::new(p.ADC, Irqs, adc::Config::default());
    let horn = adc::Channel::new_pin(p.PIN_26, Pull::None);
    let brake = adc::Channel::new_pin(p.PIN_27, Pull::None);
    let pot_drv = Output::new(p.PIN_19, Level::Low);
    spawner.spawn(defmt::unwrap!(read_potentiometers(
        adc, brake, horn, pot_drv
    )));

    spawner.spawn(defmt::unwrap!(read_function_buttons(UserInputs {
        user: [
            // User 1-4
            user1,
            Input::new(p.PIN_20, Pull::Down),
            Input::new(p.PIN_18, Pull::Down),
            Input::new(p.PIN_2, Pull::Down),
            // Everything else...
            Input::new(p.PIN_16, Pull::Down), // Bell
            Input::new(p.PIN_17, Pull::Down), // Dynamics
        ],
        profile: RotarySwitch::new(p.PIN_13, p.PIN_12, p.PIN_11, p.PIN_10)
    })));

    spawner.spawn(defmt::unwrap!(read_triple_switches(
        TripleSwitchInputs::new(p.PIN_7, p.PIN_8, p.PIN_9)
    )));

    while let Err(err) = control
        .join(
            &wifi_config.ssid,
            match wifi_config.password {
                Some(ref password) => JoinOptions::new(password.as_bytes()),
                None => JoinOptions::new_open(),
            },
        )
        .await
    {
        info!("join failed with status={}", err);
        Timer::after(Duration::from_secs(5)).await;
    }

    spawner.spawn(defmt::unwrap!(net_task(runner)));

    info!("waiting for link...");
    stack.wait_link_up().await;

    info!("waiting for DHCP...");
    stack.wait_config_up().await;

    // And now we can use it!
    info!("Stack is up!");
    connected_light.set_high();
    critical_section::with(|cs| {
        if let Some(output) = STATUS_LIGHT.borrow_ref_mut(cs).as_mut() {
            output.set_low();
        }
    });

    // And now we can use it!

    static LINE_BUFFER: StaticCell<[u8; 4096]> = StaticCell::new();
    let line_buffer = LINE_BUFFER.init([0; 4096]);
    static ROSTER_BUFFER: StaticCell<heapless::String<4096>> = StaticCell::new();
    let roster_buffer = ROSTER_BUFFER.init(Default::default());
    static LOCOMOTIVE_BUFFER: StaticCell<heapless::String<4096>> = StaticCell::new();
    let locomotive_buffer = LOCOMOTIVE_BUFFER.init(Default::default());

    static HEARTBEAT_INTERVAL: Signal<CriticalSectionRawMutex, Duration> = Signal::new();
    spawner.spawn(defmt::unwrap!(withrottle_heartbeat(&HEARTBEAT_INTERVAL)));

    loop {
        let client_state = TcpClientState::<1, 4096, 4096>::new();
        let tcp_client = TcpClient::new(stack, &client_state);
        let dns_client = DnsSocket::new(stack);

        let discovery = critical_section::with(|cs| {
            let config = config.borrow_ref(cs);
            config.withrottle_server.discovery.clone()
        });

        let withrottle_server = match discovery {
            WiThrottleDiscovery::Hardcoded(address) => address,
            WiThrottleDiscovery::Mdns => {
                // This doesn't actually work yet, I need SRV records:
                // https://github.com/smoltcp-rs/smoltcp/pull/1151
                match dns_client
                    .query("_withrottle._tcp.local", DnsQueryType::A)
                    .await
                    .map(|results| results.into_iter().next())
                {
                    Ok(Some(server)) => SocketAddr::new(server.into(), 12090),
                    Ok(None) => {
                        defmt::error!("No withrottle servers found!");
                        Timer::after(Duration::from_secs(1)).await;
                        continue;
                    }
                    Err(err) => {
                        defmt::error!("Failed to lookup withrottle servers via mdns {}", err);
                        Timer::after(Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }
        };
        info!("Connecting to withrottle server: {}", withrottle_server);
        let withrottle_socket = match tcp_client.connect(withrottle_server).await {
            Ok(socket) => {
                defmt::info!("Connected to withrottle sever at {}!", withrottle_server);
                socket
            }
            Err(err) => {
                defmt::error!(
                    "Couldn't connect to withrottle server at {}: {}",
                    withrottle_server,
                    err
                );
                Timer::after(Duration::from_secs(1)).await;
                continue;
            }
        };
        let hardware_address = stack.hardware_address();
        let hardware_address_bytes = hardware_address.as_bytes();
        let mut id = [0u8; 32];
        let id = base16ct::lower::encode(hardware_address_bytes, &mut id).unwrap();

        let mut withrottle_client = match WiThrottleClient::new(
            withrottle_socket,
            id,
            ProfileWrapper {
                mutex: config,
                profile_index: 0,
            },
            line_buffer,
            roster_buffer,
            locomotive_buffer,
            &HEARTBEAT_INTERVAL,
        )
        .await
        {
            Ok(client) => client,
            Err(err) => {
                error!("Failed to connect to withrottle server: {}", err);
                continue;
            }
        };

        connected_light.set_high();
        Timer::after(Duration::from_secs(1)).await;
        connected_light.set_low();

        'client: loop {
            while let Ok(message) = WITHROTTLE_COMMAND_CHANNEL.try_receive() {
                info!("Got a message from the command channel: {}", message);
                if let WiThrottleRequest::Disconnect = message {
                    defmt::info!("Disconnecting due to command channel message...");
                    break 'client;
                }
                let result = async {
                    match message {
                        WiThrottleRequest::SetFunctionState(function_id, state) => {
                            withrottle_client
                                .set_function_state(function_id, state)
                                .await?;
                            defmt::info!("Set the state of function {} to {}", function_id, state);
                        }
                        WiThrottleRequest::SetProfile(profile) => {
                            withrottle_client.set_profile(profile).await?;
                            defmt::info!("Set profile to #{}", profile);
                        }
                        WiThrottleRequest::SetReverser(direction) => {
                            withrottle_client.set_direction(direction).await?;
                        }
                        WiThrottleRequest::SetThrottle(position) => {
                            withrottle_client.set_speed((position as u8) * 15).await?;
                        }
                        WiThrottleRequest::Heartbeat => {
                            withrottle_client.heartbeat().await?;
                        }
                        WiThrottleRequest::Disconnect => defmt::unreachable!(),
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
