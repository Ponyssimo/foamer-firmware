use core::cell::RefCell;
use critical_section::Mutex;
use defmt::Format;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver as RpDriver;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_usb::{
    Builder,
    class::web_usb::Config as WebUsbConfig,
    driver::Driver,
    driver::{Endpoint, EndpointError, EndpointIn, EndpointOut},
};
use foamer_types::Config;
use heapless::Vec;

use crate::flash::FlashCommand;
use crate::profile_usb_types::{InControlMessage, OutControlMessage};

pub struct ProfileUsbHandler<'d, D: Driver<'d>> {
    endpoints: ProfileUsbEndpoints<'d, D>,
    config: &'static Mutex<RefCell<Config>>,
    flash_channel_tx: Sender<'static, CriticalSectionRawMutex, FlashCommand, 1>,
}

pub struct ProfileUsbEndpoints<'d, D: Driver<'d>> {
    write_endpoint: D::EndpointIn,
    read_endpoint: D::EndpointOut,
    max_packet_size: u16,
}

impl<'d, D: Driver<'d>> ProfileUsbEndpoints<'d, D> {
    pub fn new(webusb_config: &WebUsbConfig<'d>, builder: &mut Builder<'d, D>) -> Self {
        let mut function = builder.function(0xff, 0x00, 0x00);
        let mut interface = function.interface();
        let mut alt = interface.alt_setting(0xff, 0x00, 0x00, None);

        let write_endpoint = alt.endpoint_bulk_in(None, webusb_config.max_packet_size);
        let read_endpoint = alt.endpoint_bulk_out(None, webusb_config.max_packet_size);
        Self {
            write_endpoint,
            read_endpoint,
            max_packet_size: webusb_config.max_packet_size,
        }
    }
}

impl<'d, D: Driver<'d>> ProfileUsbHandler<'d, D> {
    pub fn new(
        endpoints: ProfileUsbEndpoints<'d, D>,
        config: &'static Mutex<RefCell<Config>>,
        flash_channel_tx: Sender<'static, CriticalSectionRawMutex, FlashCommand, 1>,
    ) -> Self {
        Self {
            endpoints,
            config,
            flash_channel_tx,
        }
    }

    async fn run(&mut self) -> ! {
        loop {
            self.endpoints.read_endpoint.wait_enabled().await;
            defmt::info!("Connected to USB!");
            match self.run_connected().await {
                Err(err) => {
                    defmt::error!("Failed to run connected loop... {}", err);
                }
            }
        }
    }

    async fn read<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8], EndpointError> {
        let length = self.endpoints.read_endpoint.read(buf).await?;
        Ok(&buf[..length])
    }

    async fn run_connected(&mut self) -> Result<core::convert::Infallible, RunError> {
        let mut buf = [0u8; 64];
        loop {
            let message = self.read(&mut buf).await?;
            let control_message: OutControlMessage = postcard::from_bytes(message)?;
            match control_message {
                OutControlMessage::WriteConfig { length } => {
                    self.consume_remote_config(length).await?;
                }
                OutControlMessage::ReadConfig => {
                    // Try 2kB buffer on the stack
                    // You surely will not regret 2kB buffer on the stack
                    let mut buf = [0u8; 2048];
                    let config_buf = critical_section::with(|cs| {
                        postcard::to_slice(&*self.config.borrow_ref(cs), &mut buf)
                    })?;
                    postcard::to_slice(
                        &InControlMessage::ReadConfig {
                            length: config_buf.len(),
                        },
                        &mut buf,
                    )?;
                    for chunk in buf.chunks(self.endpoints.max_packet_size.into()) {
                        self.endpoints.write_endpoint.write(chunk).await?;
                    }
                }
            }
        }
    }

    async fn consume_remote_config(&mut self, remote_config_length: usize) -> Result<(), RunError> {
        self.flash_channel_tx
            .send(FlashCommand::Reset {
                length: remote_config_length,
            })
            .await;

        let mut buf: Vec<u8, 64> = Vec::from_array([0; 64]);
        let mut cumulative_length = 0;
        while cumulative_length < remote_config_length {
            let chunk = self.read(&mut buf).await?;
            let length = chunk.len();
            cumulative_length += length;
            self.flash_channel_tx
                .send(FlashCommand::WriteChunk {
                    data: {
                        let mut buf = buf.clone();
                        buf.truncate(length);
                        buf
                    },
                })
                .await;
        }
        Ok(())
    }
}

#[derive(Format)]
enum RunError {
    Endpoint(EndpointError),
    Postcard(postcard::Error),
}

impl From<postcard::Error> for RunError {
    fn from(error: postcard::Error) -> Self {
        RunError::Postcard(error)
    }
}
impl From<EndpointError> for RunError {
    fn from(error: EndpointError) -> Self {
        RunError::Endpoint(error)
    }
}

#[embassy_executor::task]
pub async fn usb_task(
    endpoints: ProfileUsbEndpoints<'static, RpDriver<'static, USB>>,
    config: &'static Mutex<RefCell<Config>>,
    flash_channel_tx: Sender<'static, CriticalSectionRawMutex, FlashCommand, 1>,
) -> ! {
    let mut handler = ProfileUsbHandler::new(endpoints, config, flash_channel_tx);
    handler.run().await
}
