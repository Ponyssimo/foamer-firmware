use crate::buf_reader::{BufReader, BufReaderError, ReadLineError};
use crate::profile::{Address, Config, Function, PROFILE_FUNCTION_COUNT, Profile};
use crate::{CancellationSignal, ReverserPosition};
use core::cell::RefCell;
use critical_section::Mutex;
use defmt::Format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Duration;
use embedded_io_async::{Read, ReadExactError, Write};

impl Address {
    fn to_withrottle(self) -> heapless::String<5> {
        match self {
            Self::Long(long) => {
                defmt::unwrap!(heapless::format!(5; "L{:04x}", long), "encode long")
            }
            Self::Short(short) => {
                defmt::unwrap!(heapless::format!(5; "S{:02x}", short), "encode short")
            }
        }
    }
}

#[derive(Format, Default, Clone)]
struct FunctionData {
    /// ID of the function for this locomotive in particular
    withrottle_id: Option<usize>,
    /// State the function is in (pressed or not), so we can copy it when we
    /// swap profiles or otherwise reconnect
    state: bool,
}

pub struct ProfileWrapper {
    pub mutex: &'static Mutex<RefCell<Config>>,
    pub profile_index: usize,
}
impl ProfileWrapper {
    fn with<T>(&self, f: impl FnMut(&Profile) -> T) -> T {
        self.with_profile_at_index(self.profile_index, f)
    }
    fn with_profile_at_index<T>(
        &self,
        profile_index: usize,
        mut f: impl FnMut(&Profile) -> T,
    ) -> T {
        critical_section::with(|cs| {
            let config = self.mutex.borrow_ref(cs);
            f(&config.profiles[profile_index])
        })
    }
}

pub struct WiThrottleClient<'a, Conn: Read + Write> {
    functions: [FunctionData; PROFILE_FUNCTION_COUNT],
    connection: BufReader<Conn>,
    profile: ProfileWrapper,
    address: Address,
    line_buffer: &'a mut [u8; 4096],
    roster_buffer: &'a mut heapless::String<4096>,
    locomotive_buffer: &'a mut heapless::String<4096>,
    heartbeat_interval: &'a Signal<CriticalSectionRawMutex, Duration>,

    direction: ReverserPosition,
    speed: u8,
}

#[derive(Format, Clone)]
pub enum WiThrottleError {
    ProtocolError,
    ReadError,
    Cancelled,
}

impl<'a, T: embedded_io_async::ErrorType> From<ReadLineError<'a, T>> for WiThrottleError {
    fn from(error: ReadLineError<'a, T>) -> Self {
        match error {
            ReadLineError::ReadExactError(error) => Self::from(error),
            ReadLineError::Cancelled => Self::Cancelled,
            ReadLineError::NoNewline(_, _) => Self::ProtocolError,
        }
    }
}

impl<T> From<ReadExactError<T>> for WiThrottleError {
    fn from(_error: ReadExactError<T>) -> Self {
        Self::ProtocolError
    }
}
impl<T: Format + embedded_io_async::Error + core::fmt::Debug> From<BufReaderError<T>>
    for WiThrottleError
{
    fn from(_error: BufReaderError<T>) -> Self {
        Self::ReadError
    }
}

impl<'a, Conn: Read + Write> WiThrottleClient<'a, Conn>
where
    Conn::Error: Format,
{
    pub async fn new(
        connection: Conn,
        id: &[u8],
        profile: ProfileWrapper,
        line_buffer: &'a mut [u8; 4096],
        roster_buffer: &'a mut heapless::String<4096>,
        locomotive_buffer: &'a mut heapless::String<4096>,
        heartbeat_interval: &'a Signal<CriticalSectionRawMutex, Duration>,
    ) -> Result<Self, WiThrottleError> {
        roster_buffer.clear();
        locomotive_buffer.clear();

        let mut this = Self {
            connection: connection.into(),
            address: profile.with(|profile| profile.address),
            profile,
            functions: Default::default(),
            line_buffer,
            roster_buffer,
            locomotive_buffer,
            heartbeat_interval,
            speed: 0,
            direction: Default::default(),
        };
        this.reset_function_mapping();

        // Idenitfy ourselves (ID)
        this.connection.write_all(b"HU").await?;
        this.connection.write_all(id).await?;
        this.connection.write_all(b"\n").await?;
        // Name
        this.connection.write_all(b"Nfoamer-").await?;
        this.connection.write_all(id).await?;
        this.connection.write_all(b"\n").await?;
        // Enable heartbeats, heartbeat
        this.connection.write_all(b"*+\n").await?;
        this.heartbeat().await?;

        if this
            .read_line()
            .await
            .map(|line| line != *b"VN2.0")
            .unwrap_or(true)
        {
            defmt::error!("Expected version 2.0, got something else");
            return Err(WiThrottleError::ProtocolError);
        }

        Ok(this)
    }

    pub async fn set_profile(&mut self, profile_index: usize) -> Result<(), WiThrottleError> {
        let new_locomotive = self.address
            != self
                .profile
                .with_profile_at_index(profile_index, |profile| profile.address);
        if new_locomotive {
            self.locomotive_buffer.clear();
            self.remove_locomotive().await?;
        }

        self.profile.profile_index = profile_index;
        self.address = self.profile.with(|profile| profile.address);

        self.reset_function_mapping();

        if new_locomotive {
            // Try and find our locomotive in the roster
            self.handle_roster().await?;
        } else {
            // Profile function mapping is probably different
            self.handle_locomotive().await?;
        }

        Ok(())
    }

    pub async fn handle_line(
        &mut self,
        cancellation_signal: Option<&CancellationSignal>,
    ) -> Result<(), WiThrottleError> {
        let line = self
            .connection
            .read_line(self.line_buffer, cancellation_signal)
            .await?;
        let line = str::from_utf8(line).map_err(|_| WiThrottleError::ProtocolError)?;

        if let Some(line) = line.strip_prefix("*") {
            self.heartbeat_interval.signal(Duration::from_secs(
                line.parse::<u64>()
                    .map_err(|_| WiThrottleError::ProtocolError)?
                    / 2u64,
            ));
            self.heartbeat().await?;
            defmt::info!("Ran heartbeat");
        } else if let Some(line) = line.strip_prefix("M0L") {
            defmt::info!("This is info about a locomotive! {}", line);
            let type_char = defmt::unwrap!(line.chars().next());
            let (line, number) = defmt::unwrap!(Self::read_number(&line[1..], 16));
            let address = match type_char {
                'L' => Address::Long(defmt::unwrap!(number.try_into())),
                'S' => Address::Short(defmt::unwrap!(number.try_into())),
                _ => defmt::unimplemented!(),
            };
            // I don't care about this one
            if address != self.address {
                return Ok(());
            }
            defmt::info!("This is info about OUR locomotive! {} {}", address, line);
            defmt::assert_eq!(&line[0..6], "<;>]\\[");

            let list = &line[6..line.len() - 3];
            self.locomotive_buffer.clear();
            defmt::unwrap!(
                self.locomotive_buffer.push_str(list),
                "Somehow locomotive buffer was smaller than line?"
            );
            self.handle_locomotive().await?;
        } else if let Some(line) = line.strip_prefix("RL") {
            self.roster_buffer.clear();
            defmt::unwrap!(
                self.roster_buffer.push_str(line),
                "Somehow roster buffer was smaller than line?"
            );
            self.handle_roster().await?;
        } else {
            defmt::warn!("Unknown command: {}", line);
        }

        Ok(())
    }

    fn reset_function_mapping(&mut self) {
        defmt::trace!("Resetting function mapping...");
        for function in self.functions.iter_mut() {
            function.withrottle_id = None;
        }
    }

    async fn handle_locomotive(&mut self) -> Result<(), WiThrottleError> {
        self.reset_function_mapping();

        if self.locomotive_buffer.is_empty() {
            defmt::info!(
                "Tried to handle locomotive, but it doesn't look like we have one in our buffer"
            );
            return Ok(());
        }
        let list = self.locomotive_buffer.as_str();
        let mut profile_function_needs_update = [false; PROFILE_FUNCTION_COUNT];
        self.profile.with(|profile| {
            for (index, function_label) in list.split("]\\[").enumerate() {
                for (profile_index, profile_function) in profile.functions.iter().enumerate() {
                    if let Some(Function::Label {
                        label: profile_function,
                        momentary: _,
                    }) = profile_function
                        && profile_function == function_label
                    {
                        defmt::info!(
                            "Found info about a function we care about: {}. It is at {}",
                            function_label,
                            index
                        );
                        self.functions[profile_index].withrottle_id = Some(index);
                        profile_function_needs_update[profile_index] = true;
                    }
                }
            }
        });

        for profile_index in profile_function_needs_update
            .into_iter()
            .enumerate()
            .filter_map(|(index, needs_update)| if needs_update { Some(index) } else { None })
        {
            self.send_function_state(profile_index).await?;
        }

        Ok(())
    }

    async fn handle_roster(&mut self) -> Result<(), WiThrottleError> {
        if self.roster_buffer.is_empty() {
            defmt::warn!(
                "Tried to handle roster, but it doesn't look like we have one in our buffer"
            );
            return Ok(());
        }

        let line = self.roster_buffer.as_str();
        let (line, count) = defmt::unwrap!(Self::read_number(&line[0..], 10));
        defmt::info!("Got {} roster entries!", count);
        if count > 0 {
            let line = defmt::unwrap!(line.strip_prefix("]\\["));
            for roster_entry in line.splitn(count, "]\\[") {
                defmt::info!("Working roster entry.. {}", roster_entry);
                let mut roster_entry = roster_entry.splitn(3, "}|{");
                let name = roster_entry.next().ok_or(WiThrottleError::ProtocolError)?;
                let address = roster_entry.next().ok_or(WiThrottleError::ProtocolError)?;
                let (address_line, address) =
                    Self::read_number(address, 16).ok_or(WiThrottleError::ProtocolError)?;
                if !address_line.is_empty() {
                    defmt::error!(
                        "Found extra junk at the end of the address: {}",
                        address_line
                    );
                    return Err(WiThrottleError::ProtocolError);
                }
                let address_length = roster_entry.next().ok_or(WiThrottleError::ProtocolError)?;
                let address = match address_length {
                    "L" => Address::Long(address as u16),
                    "S" => Address::Short(address as u8),
                    address_length => {
                        defmt::error!("Unknown address length {}", address_length);
                        return Err(WiThrottleError::ProtocolError);
                    }
                };
                if self.address == address {
                    defmt::info!("This roster entry is us! {} / {}", name, address);
                    self.add_locomotive(address).await?;
                    break;
                }
            }
        } else {
            let address = self.address;
            defmt::info!(
                "Manually adding locomotive at address {}... I hate this system",
                address
            );
            self.add_locomotive(address).await?;
        }
        Ok(())
    }

    async fn remove_locomotive(&mut self) -> Result<(), WiThrottleError> {
        self.write_throttle().await?;
        self.connection.write_all(b"-*<;>r\n").await?;

        Ok(())
    }

    async fn add_locomotive(&mut self, address: Address) -> Result<(), WiThrottleError> {
        self.write_throttle().await?;
        self.connection.write_all(b"+").await?;
        self.connection
            .write_all(address.to_withrottle().as_bytes())
            .await?;
        self.connection.write_all(b"<;>").await?;
        self.connection
            .write_all(address.to_withrottle().as_bytes())
            .await?;
        self.connection.write_all(b"\n").await?;

        // Set speed step to 27 notches
        self.write_locomotive_action().await?;
        self.connection.write_all(b"s1\n").await?;

        // self.write_locomotive_action().await?;
        // self.connection.write_all(b"m").await?;
        // self.connection.write_all(b"1").await?;
        // if let Some(self.profile.functions

        // Implicitly sends speed update too
        self.set_direction(self.direction).await?;

        for function_id in 0..self.functions.len() {
            self.send_function_state(function_id).await?;
        }

        Ok(())
    }

    fn read_number(number: &str, radix: u32) -> Option<(&str, usize)> {
        let mut index = 0;
        for character in number.chars() {
            match character {
                '0'..='9' => {
                    index += 1;
                    continue;
                }
                _ => {
                    break;
                }
            }
        }
        usize::from_str_radix(&number[0..index], radix)
            .ok()
            .map(|num| (&number[index..], num))
    }

    async fn write_throttle(&mut self) -> Result<(), WiThrottleError> {
        self.connection.write_all(b"M0").await.map_err(Into::into)
    }

    async fn write_locomotive_action(&mut self) -> Result<(), WiThrottleError> {
        self.write_throttle().await?;
        self.connection.write_all(b"A").await?;
        self.connection
            .write_all(self.address.to_withrottle().as_bytes())
            .await?;
        self.connection.write_all(b"<;>").await?;
        Ok(())
    }

    async fn write_function_state(
        &mut self,
        withrottle_function_id: usize,
        state: bool,
        momentary: bool,
    ) -> Result<(), WiThrottleError> {
        self.write_locomotive_action().await?;
        self.connection
            .write_all(&[b'm', if momentary { b'1' } else { b'0' }])
            .await?;
        self.connection
            .write_all(
                defmt::unwrap!(
                    heapless::format!(20; "{withrottle_function_id}\n"),
                    "Format function id"
                )
                .as_bytes(),
            )
            .await?;
        // self.connection
        //     .write_all(
        //         defmt::unwrap!(
        //             heapless::format!(20; "m{momentary}{withrottle_function_id}\n"),
        //             "Format function id for momentary"
        //         )
        //         .as_bytes(),
        //     )
        //     .await?;

        self.write_locomotive_action().await?;
        self.connection
            .write_all(&[b'F', if state { b'1' } else { b'0' }])
            .await?;
        self.connection
            .write_all(
                defmt::unwrap!(
                    heapless::format!(20; "{withrottle_function_id}"),
                    "Format function id"
                )
                .as_bytes(),
            )
            .await?;
        self.connection.write_all(b"\n").await?;
        Ok(())
    }

    async fn send_function_state(&mut self, function_id: usize) -> Result<(), WiThrottleError> {
        let function = &self.functions[function_id];
        // Unnecessary clone of the label, but boohoo
        match self
            .profile
            .with(|profile| profile.functions[function_id].clone())
        {
            Some(Function::Label {
                label: _,
                momentary,
            }) => {
                if let Some(function_id) = function.withrottle_id {
                    self.write_function_state(function_id, function.state, momentary)
                        .await?;
                }
            }
            Some(Function::Hardcoded {
                id: function_id,
                momentary,
            }) => {
                self.write_function_state(function_id.into(), function.state, momentary)
                    .await?;
            }
            Some(Function::EmergencyStop) if function.state => {
                self.write_locomotive_action().await?;
                self.connection.write_all(b"X").await?;
            }
            Some(Function::EmergencyStop) | None => {}
        }
        Ok(())
    }

    pub async fn set_function_state(
        &mut self,
        button_id: usize,
        state: bool,
    ) -> Result<(), WiThrottleError> {
        let function = &mut self.functions[button_id];
        function.state = state;
        self.send_function_state(button_id).await?;
        Ok(())
    }

    pub async fn send_speed(&mut self) -> Result<(), WiThrottleError> {
        let speed = if self.direction == ReverserPosition::Neutral {
            0
        } else {
            self.speed
        };
        self.write_locomotive_action().await?;
        self.connection.write_all(b"V").await?;
        self.connection
            .write_all(defmt::unwrap!(heapless::format!(5; "{speed}"), "Format speed").as_bytes())
            .await?;
        self.connection.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn set_speed(&mut self, speed: u8) -> Result<(), WiThrottleError> {
        self.speed = speed;
        self.send_speed().await?;
        Ok(())
    }

    pub async fn set_direction(
        &mut self,
        direction: ReverserPosition,
    ) -> Result<(), WiThrottleError> {
        self.direction = direction;
        let direction = match direction {
            ReverserPosition::Reverse => Some(b"0"),
            ReverserPosition::Forwards => Some(b"1"),
            ReverserPosition::Neutral => None,
        };
        if let Some(direction) = direction {
            self.write_locomotive_action().await?;
            self.connection.write_all(b"R").await?;
            self.connection.write_all(direction).await?;
            self.connection.write_all(b"\n").await?;
        }
        self.send_speed().await?;
        Ok(())
    }

    pub async fn heartbeat(&mut self) -> Result<(), WiThrottleError> {
        self.connection.write_all(b"*\n").await?;
        Ok(())
    }

    async fn read_line<const N: usize>(&mut self) -> Result<[u8; N], WiThrottleError> {
        let mut buffer = [0u8; N];
        let buf_ptr = self.connection.read_line(&mut buffer, None).await?;
        if buf_ptr.len() != N {
            defmt::error!(
                "Didn't get expected length of {}. Instead got {}",
                N,
                buf_ptr
            );
            return Err(WiThrottleError::ProtocolError);
        }
        Ok(buffer)
    }

    async fn read_bytes<const N: usize>(
        &mut self,
    ) -> Result<[u8; N], ReadExactError<BufReaderError<Conn::Error>>> {
        let mut result = [0u8; N];
        self.connection.read_exact(&mut result).await?;
        Ok(result)
    }
}
