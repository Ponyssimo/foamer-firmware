use crate::buf_reader::{BufReader, BufReaderError, ReadLineError};
use crate::profile::{Address, Profile};
use crate::{
    CancellationSignal, ReverserPosition, TRIPLE_SWITCH_FUNCTION_COUNT, TRIPLE_SWITCHES,
    USER_BUTTONS,
};
use defmt::Format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant};
use embedded_io_async::{Read, ReadExactError, Write};

impl Address {
    fn to_withrottle(self) -> heapless_latest::String<5> {
        match self {
            Self::Long(long) => {
                defmt::unwrap!(heapless_latest::format!(5; "L{:04x}", long), "encode long")
            }
            Self::Short(short) => {
                defmt::unwrap!(
                    heapless_latest::format!(5; "S{:02x}", short),
                    "encode short"
                )
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

pub struct WiThrottleClient<'a, Conn: Read + Write> {
    functions: [FunctionData; USER_BUTTONS + (TRIPLE_SWITCHES * TRIPLE_SWITCH_FUNCTION_COUNT)],
    connection: BufReader<Conn>,
    profile: &'a Profile,
    line_buffer: &'a mut [u8; 4096],
    roster_buffer: &'a mut heapless_latest::String<4096>,
    heartbeat_interval: Duration,
    heartbeat_deadline: &'a Signal<CriticalSectionRawMutex, Instant>,

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
        profile: &'a Profile,
        line_buffer: &'a mut [u8; 4096],
        roster_buffer: &'a mut heapless_latest::String<4096>,
        heartbeat_deadline: &'a Signal<CriticalSectionRawMutex, Instant>,
    ) -> Result<Self, WiThrottleError> {
        roster_buffer.clear();

        let mut this = Self {
            connection: connection.into(),
            profile,
            functions: Default::default(),
            line_buffer,
            roster_buffer,
            heartbeat_interval: Duration::MAX,
            heartbeat_deadline,
            speed: 0,
            direction: Default::default(),
        };

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

    pub async fn set_profile(&mut self, profile: &'static Profile) -> Result<(), WiThrottleError> {
        if self.profile.address != profile.address {
            self.remove_locomotive().await?;
        }

        self.profile = profile;
        self.handle_roster().await?;

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
            self.heartbeat_interval =
                Duration::from_secs(line.parse().map_err(|_| WiThrottleError::ProtocolError)?);
            self.heartbeat().await?;
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
            if address != self.profile.address {
                return Ok(());
            }
            defmt::info!("This is info about OUR locomotive! {} {}", address, line);
            defmt::assert_eq!(&line[0..6], "<;>]\\[");
            let list = &line[6..line.len() - 3];
            for function in self.functions.iter_mut() {
                function.withrottle_id = None;
            }
            for (index, function_label) in list.split("]\\[").enumerate() {
                for (profile_index, profile_function) in self.profile.functions.iter().enumerate() {
                    if let Some(profile_function) = profile_function
                        && profile_function.label == function_label
                    {
                        defmt::info!(
                            "Found info about a function we care about: {}. It is at {}",
                            function_label,
                            index
                        );
                        self.functions[profile_index].withrottle_id = Some(index);
                    }
                }
                // ba
            }
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
                if self.profile.address == address {
                    defmt::info!("This roster entry is us! {} / {}", name, address);
                    self.add_locomotive(address).await?;
                    break;
                }
            }
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

        self.set_direction(self.direction).await?;
        self.set_speed(self.speed).await?;

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
            .write_all(self.profile.address.to_withrottle().as_bytes())
            .await?;
        self.connection.write_all(b"<;>").await?;
        Ok(())
    }

    pub async fn set_function_state(
        &mut self,
        button_id: usize,
        state: bool,
    ) -> Result<(), WiThrottleError> {
        let function = &mut self.functions[button_id];
        function.state = state;
        if let Some(function_id) = function.withrottle_id {
            self.write_locomotive_action().await?;
            self.connection
                .write_all(&[b'F', if state { b'1' } else { b'0' }])
                .await?;
            self.connection
                .write_all(
                    defmt::unwrap!(
                        heapless_latest::format!(20; "{function_id}"),
                        "Format function id"
                    )
                    .as_bytes(),
                )
                .await?;
        }
        Ok(())
    }

    pub async fn set_speed(&mut self, speed: u8) -> Result<(), WiThrottleError> {
        self.speed = speed;
        self.write_locomotive_action().await?;
        self.connection.write_all(b"V").await?;
        self.connection
            .write_all(
                defmt::unwrap!(heapless_latest::format!(5; "{speed}"), "Format speed").as_bytes(),
            )
            .await?;
        self.connection.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn set_direction(
        &mut self,
        direction: ReverserPosition,
    ) -> Result<(), WiThrottleError> {
        self.direction = direction;
        let direction = match direction {
            ReverserPosition::Reverse => b"0",
            ReverserPosition::Forwards => b"1",
            _ => return Ok(()),
        };
        self.write_locomotive_action().await?;
        self.connection.write_all(b"R").await?;
        self.connection.write_all(direction).await?;
        self.connection.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn heartbeat(&mut self) -> Result<(), WiThrottleError> {
        self.heartbeat_deadline.signal(
            Instant::now()
                .checked_add(self.heartbeat_interval)
                .unwrap_or(Instant::MAX),
        );
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
