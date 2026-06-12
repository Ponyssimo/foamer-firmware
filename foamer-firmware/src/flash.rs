use core::cell::RefCell;
use critical_section::Mutex;
use defmt::Format;
pub use embassy_rp::flash::Blocking;
use embassy_rp::flash::{Flash, Instance, Mode};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use foamer_types::Config;
use heapless::Vec;
use static_cell::StaticCell;

const PAGE_SIZE: usize = 4096;
// (2 MB... 512 pages of `PAGE_SIZE` each)
pub const FLASH_SIZE: usize = 512 * PAGE_SIZE;
type PersonalizationSector = [u8; PAGE_SIZE];
unsafe extern "C" {
    static PERSONALIZATION_SECTOR: PersonalizationSector;
    static FLASH_START: core::ffi::c_void;
}

pub fn read_config<'d, T: Instance, M: Mode, const FLASH_SIZE: usize>(
    _flash: &mut Flash<'d, T, M, FLASH_SIZE>,
) -> Result<Config, postcard::Error> {
    // Safety: We hold the one and only Flash instance
    defmt::info!("Here are a few bytes of the personalization sector {}", unsafe {&PERSONALIZATION_SECTOR[0..16]});
    postcard::from_bytes(unsafe { &PERSONALIZATION_SECTOR })
}

#[derive(Format)]
pub enum FlashCommand {
    WriteChunk { data: Vec<u8, 64> },
    Reset { length: usize },
}

#[embassy_executor::task]
pub async fn flash_task(
    mut flash: Flash<'static, FLASH, Blocking, FLASH_SIZE>,
    config: &'static Mutex<RefCell<Config>>,
    flash_channel: Receiver<'static, CriticalSectionRawMutex, FlashCommand, 1>,
) -> ! {
    const CONFIG_BUFFER_SIZE: usize = core::mem::size_of::<PersonalizationSector>();
    static REMOTE_CONFIG_BUFFER: StaticCell<Vec<u8, CONFIG_BUFFER_SIZE>> = StaticCell::new();
    let remote_config_buffer = REMOTE_CONFIG_BUFFER.init(Vec::new());
    let mut config_length = 0;

    loop {
        match flash_channel.receive().await {
            FlashCommand::Reset { length } => {
                defmt::info!("Resetting config buffer for length {}", length);
                remote_config_buffer.clear();
                config_length = length;
            }
            FlashCommand::WriteChunk { data } => {
                defmt::info!("Writing chunk of length {}: {}", data.len(), data);
                defmt::unwrap!(
                    remote_config_buffer.extend_from_slice(&data),
                    "Missing a reset somewhere in here? Someone tried to send really big data to us..."
                );
                core::mem::drop(data);
                if remote_config_buffer.len() >= config_length {
                    defmt::info!(
                        "Hit the right length! (Wanted {}, got {})",
                        config_length,
                        remote_config_buffer.len()
                    );

                    let new_config: Config = match postcard::from_bytes(&remote_config_buffer[0..config_length]) {
                        Ok(new_config) => new_config,
                        Err(err) => {
                            // TODO: Mary bubble the error back up to the client
                            defmt::error!("Bad config: {}", err);
                            continue;
                        }
                    };
                    defmt::info!("Deserialized config!");

                    // Commit!
                    critical_section::with(|cs| {
                        let mut config = config.borrow_ref_mut(cs);
                        if *config == new_config {
                            defmt::info!(
                                "New config is the same as the old. Saving some flash cycles."
                            );
                            return;
                        }
                        *config = new_config;
                        defmt::info!("Applied new config!");
                        let offset = unsafe {
                            PERSONALIZATION_SECTOR.as_ptr() as u32
                                - &FLASH_START as *const core::ffi::c_void as u32
                        };
                        defmt::unwrap!(flash.blocking_erase(offset, offset + core::mem::size_of::<PersonalizationSector>() as u32));
                        defmt::unwrap!(flash.blocking_write(
                            offset,
                            &remote_config_buffer[0..config_length],
                        ));
                        defmt::info!("Saved new config!");
                    });
                }
            }
        }
    }
}
