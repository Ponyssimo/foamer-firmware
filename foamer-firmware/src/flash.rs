use core::cell::RefCell;
use critical_section::Mutex;
use defmt::Format;
pub use embassy_rp::flash::Blocking;
use embassy_rp::flash::{Error, Flash, Instance, Mode};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use embedded_io::ErrorKind;
use foamer_types::Config;
use heapless::Vec;
use static_cell::StaticCell;

struct FlashAdapter<'a, 'd, T: Instance, M: Mode, const FLASH_SIZE: usize> {
    flash: &'a mut Flash<'d, T, M, FLASH_SIZE>,
    offset: u32,
}

#[derive(Debug, Format)]
struct FlashError {
    error: Error,
}

impl From<Error> for FlashError {
    fn from(error: Error) -> Self {
        Self { error }
    }
}
impl embedded_io::Error for FlashError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl<'a, 'd, T: Instance, M: Mode, const FLASH_SIZE: usize> embedded_io::ErrorType
    for FlashAdapter<'a, 'd, T, M, FLASH_SIZE>
{
    type Error = FlashError;
}
impl<'a, 'd, T: Instance, M: Mode, const FLASH_SIZE: usize> embedded_io::Read
    for FlashAdapter<'a, 'd, T, M, FLASH_SIZE>
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let end = core::cmp::min(FLASH_SIZE - self.offset as usize, buf.len());
        let buf = &mut buf[..end];
        self.flash.blocking_read(self.offset, buf)?;
        self.offset += buf.len() as u32;
        Ok(buf.len())
    }
}
impl<'a, 'd, T: Instance, M: Mode, const FLASH_SIZE: usize> embedded_io::Write
    for FlashAdapter<'a, 'd, T, M, FLASH_SIZE>
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let end = core::cmp::min(FLASH_SIZE - self.offset as usize, buf.len());
        let buf = &buf[..end];
        self.flash.blocking_write(self.offset, buf)?;
        self.offset += buf.len() as u32;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

unsafe extern "C" {
    static PERSONALIZATION_SECTOR: [u8; 2048];
    static FLASH_START: core::ffi::c_void;
}

pub fn read_config<'d, T: Instance, M: Mode, const FLASH_SIZE: usize>(
    _flash: &mut Flash<'d, T, M, FLASH_SIZE>,
) -> Result<Config, postcard::Error> {
    // Safety: We hold the one and only Flash instance
    postcard::from_bytes(unsafe { &PERSONALIZATION_SECTOR })
}
pub fn write_config<'d, T: Instance, M: Mode, const FLASH_SIZE: usize>(
    flash: &mut Flash<'d, T, M, FLASH_SIZE>,
    config: &Config,
) -> Result<(), postcard::Error> {
    postcard::to_eio(
        &config,
        FlashAdapter {
            flash,
            // Safety: This is just pointer arithmetic, not actually touching the
            // value stored inside
            offset: unsafe {
                PERSONALIZATION_SECTOR.as_ptr() as u32
                    - &FLASH_START as *const core::ffi::c_void as u32
            },
        },
    )?;
    Ok(())
}

// async fn read_config<'d, T: Instance, M: Mode, const FLASH_SIZE: usize>(
//     flash: Flash<'d, T, M, FLASH_SIZE>,
//     temp: &mut [u8],
// ) -> Result<Config, postcard::Error> {
//     Ok(postcard::from_bytes(
//     Ok(postcard::from_eio((
//         FlashAdapter {
//             flash,
//             offset: OFFSET as u32,
//         },
//         temp,
//     ))?
//     .0)
// }

#[derive(Format)]
pub enum FlashCommand {
    WriteChunk { data: Vec<u8, 64> },
    Reset { length: usize },
}

pub const FLASH_SIZE: usize = 2 * 1024 * 1024;
#[embassy_executor::task]
pub async fn flash_task(
    mut flash: Flash<'static, FLASH, Blocking, FLASH_SIZE>,
    config: &'static Mutex<RefCell<Config>>,
    flash_channel: Receiver<'static, CriticalSectionRawMutex, FlashCommand, 1>,
) -> ! {
    const CONFIG_BUFFER_SIZE: usize = core::mem::size_of::<Config>() * 2;
    static REMOTE_CONFIG_BUFFER: StaticCell<Vec<u8, CONFIG_BUFFER_SIZE>> = StaticCell::new();
    let remote_config_buffer = REMOTE_CONFIG_BUFFER.init(Vec::new());
    let mut config_length = 0;

    loop {
        match flash_channel.receive().await {
            FlashCommand::Reset { length } => {
                remote_config_buffer.clear();
                config_length = length;
            }
            FlashCommand::WriteChunk { data } => {
                defmt::unwrap!(
                    remote_config_buffer.extend_from_slice(&data),
                    "Missing a reset somewhere in here? Someone tried to send really big data to us..."
                );
                if remote_config_buffer.len() >= config_length {
                    defmt::info!("Finished receiving config! Let's decode and commit it!");
                    let new_config: Config = match postcard::from_bytes(remote_config_buffer) {
                        Ok(new_config) => new_config,
                        Err(err) => {
                            // TODO: Mary bubble the error back up to the client
                            defmt::error!("Bad config: {}", err);
                            continue;
                        }
                    };
                    defmt::info!("Config is {}", new_config);

                    // Commit!
                    critical_section::with(|cs| {
                        let mut config = config.borrow_ref_mut(cs);
                        *config = new_config.clone();
                    });

                    // This basically shouldn't happen
                    defmt::unwrap!(write_config(&mut flash, &new_config));
                }
            }
        }
    }
}
