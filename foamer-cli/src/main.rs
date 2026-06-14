use anyhow::Context;
use clap::{Parser, Subcommand};
use foamer_types::Config;
use foamer_types::profile_usb_types::{InControlMessage, OutControlMessage};
use nusb::transfer::{Bulk, In, Out};
use nusb::{MaybeFuture, list_devices};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Usb {
        #[command(subcommand)]
        action: Action,
    },
    VerifyConfig {
        config_file: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum Action {
    Dump { output_file: PathBuf },
    Program { input_config: PathBuf },
    Info,
}

fn deserialize_json<'de, T: Deserialize<'de>>(slice: &'de [u8]) -> anyhow::Result<T> {
    let deserializer = &mut serde_json::Deserializer::from_slice(slice);
    serde_path_to_error::deserialize(deserializer).map_err(|err| {
        anyhow::anyhow!(
            "Failed to parse {}: {}",
            err.path().to_string(),
            err.into_inner()
        )
    })
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Usb { action } => handle_usb(action),
        Command::VerifyConfig { config_file } => {
            let config = std::fs::read(config_file).context("Reading config file")?;
            let _: Config = deserialize_json(&config).context("Deserializing config")?;
            println!("Good config!");
            Ok(())
        }
    }
}

fn handle_usb(action: Action) -> anyhow::Result<()> {
    let device_info = list_devices()
        .wait()?
        .find(|dev| dev.vendor_id() == 0x0403 && dev.product_id() == 0x698F)
        .context("Device not found")?;

    let device = device_info.open().wait()?;
    let interface = device.claim_interface(1).wait()?;

    let mut writer = interface
        .endpoint::<Bulk, Out>(0x01)
        .context("Writer")?
        .writer(64);
    let mut reader = interface
        .endpoint::<Bulk, In>(0x81)
        .context("Reader")?
        .reader(64);
    let mut buf = [0; 64];

    match action {
        Action::Dump { output_file } => {
            let file = File::create(&output_file).context("Creating output file")?;
            println!("Writing to {}", output_file.display());
            postcard::to_io(&OutControlMessage::ReadConfig, &mut writer)
                .context("Writing out ReadConfig packet")?;
            let short_packet_reader = reader.until_short_packet();
            let response: InControlMessage = postcard::from_io((short_packet_reader, &mut buf))
                .context("Deserializing In transfer")?
                .0;
            let length = match response {
                InControlMessage::ReadConfig { length } => length,
                other => panic!("Unexpected message: {other:?}"),
            };
            let mut allocation = vec![0u8; length];
            reader
                .read_exact(&mut allocation)
                .context("Reading config chunk stream")?;
            let config: Config =
                postcard::from_bytes(&allocation).context("Deserializing received config")?;
            serde_json::to_writer_pretty(BufWriter::new(file), &config)
                .context("Writing to config file")?;
            println!("Wrote to {}!", output_file.display());
        }
        Action::Program { input_config } => {
            println!("Reading from {}", input_config.display());
            let config = std::fs::read(input_config).context("Reading input config file")?;
            let config: Config = deserialize_json(&config).context("Parsing config file")?;

            let config_bytes = postcard::to_stdvec(&config).context("Serializing config")?;
            postcard::to_io(
                &OutControlMessage::WriteConfig {
                    length: config_bytes.len(),
                },
                &mut writer,
            )
            .context("Writing config length command")?;
            writer
                .write_all(&config_bytes)
                .context("Writing config chunk to USB")?;

            println!("Sent config over. I don't get any confirmation that it worked.");
            println!("!! Remember wifi settings won't apply until you power cycle !!");
        }
        Action::Info => {
            println!(
                "USB Device found: {} - {}",
                device_info.product_string().unwrap_or("N/A"),
                device_info.serial_number().unwrap_or("N/A")
            );
        }
    }

    Ok(())
}
