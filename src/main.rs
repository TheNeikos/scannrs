use std::collections::HashMap;
use std::ffi::CStr;
use std::ffi::CString;
use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use image::codecs::jpeg::JpegEncoder;
use image::DynamicImage;
use miette::Context;
use miette::Diagnostic;
use miette::IntoDiagnostic;
use sane_scan::DeviceOptionValue;
use sane_scan::Sane;
use thiserror::Error;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List available scanners
    List,
    /// Get all options this scanner exposes
    Options {
        /// Which scanner to operate on
        name: String,

        #[command(subcommand)]
        command: Option<OptionsCommand>,
    },
    Scan {
        /// Which scanner to operate on
        name: String,

        /// Whether to scan in color or grayscale
        #[arg(short, long)]
        options: Vec<String>,

        /// The path to save the scan at
        #[arg(short, long)]
        path: PathBuf,
    },
}

#[derive(Default, Subcommand)]
enum OptionsCommand {
    #[default]
    List,
    Show {
        option: String,
    },
}

#[derive(Debug, Error, Diagnostic)]
enum ScannrsError {
    #[error("Could not find scanner with name: '{}'", .name)]
    CouldNotFindScanner { name: String },

    #[error("An error occured while communicating with the scanner: {}", .error)]
    Sane {
        #[from]
        error: sane_scan::Error,
    },
    #[error("The given option '{}' does not exist for scanner '{}'", .option, .name)]
    OptionNotFound { name: String, option: String },
}

fn main() -> miette::Result<()> {
    human_panic::setup_panic!();

    let args = Cli::parse();

    let sane = Sane::init_1_0().into_diagnostic()?;

    match args.command {
        Command::List => {
            for device in sane.get_devices().into_diagnostic()? {
                println!("{device:?}");
            }
        }
        Command::Options { name, command } => {
            let device = match sane
                .get_devices()
                .into_diagnostic()?
                .into_iter()
                .find_map(|d| (d.name.as_bytes() == name.as_bytes()).then(|| d.open()))
            {
                Some(device) => device
                    .map_err(ScannrsError::from)
                    .into_diagnostic()
                    .with_context(|| {
                        format!("While trying to open a connection with scanner {}", name)
                    })?,
                None => return Err(ScannrsError::CouldNotFindScanner { name }.into()),
            };

            match command.unwrap_or_default() {
                OptionsCommand::List => {
                    let options = device.get_options().into_diagnostic()?;

                    for option in options {
                        match option.type_ {
                            sane_scan::ValueType::Group => {
                                println!("[{}]", option.title.to_string_lossy());
                            }
                            t => {
                                println!(
                                    "# {}\n{} = {t:?}",
                                    option.title.to_string_lossy(),
                                    option.name.to_string_lossy(),
                                );
                            }
                        }
                    }
                }
                OptionsCommand::Show { option } => {
                    let options = device.get_options().into_diagnostic()?;

                    let device_option = options
                        .into_iter()
                        .find(|o| o.name.as_bytes() == option.as_bytes())
                        .ok_or_else(|| ScannrsError::OptionNotFound {
                            name: name.clone(),
                            option: option.clone(),
                        })
                        .into_diagnostic()?;

                    let value = device
                        .get_option(&device_option)
                        .into_diagnostic()
                        .with_context(|| {
                            format!(
                                "While trying to read the option '{option}' from scanner '{name}'"
                            )
                        })?;

                    println!("{value:?}");
                }
            }
        }
        Command::Scan {
            name,
            path,
            options,
        } => {
            let mut device = match sane
                .get_devices()
                .into_diagnostic()?
                .into_iter()
                .find_map(|d| (d.name.as_bytes() == name.as_bytes()).then(|| d.open()))
            {
                Some(device) => device
                    .map_err(ScannrsError::from)
                    .into_diagnostic()
                    .with_context(|| {
                        format!("While trying to open a connection with scanner {}", name)
                    })?,
                None => return Err(ScannrsError::CouldNotFindScanner { name }.into()),
            };

            let file = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&path)
                .into_diagnostic()
                .with_context(|| format!("Tried to write to file at {}", path.display()))?;

            let options: HashMap<&[u8], &str> = options
                .iter()
                .map(|option| {
                    let (key, val) = option.split_once("=").unwrap();
                    let key = key.trim();
                    let val = val.trim();
                    (key.as_bytes(), val)
                })
                .collect();

            for opt in device.get_options().into_diagnostic()? {
                if let Some(val) = options.get(opt.name.as_bytes()) {
                    let val = match opt.type_ {
                        sane_scan::ValueType::Int => {
                            DeviceOptionValue::Int(val.parse().into_diagnostic()?)
                        }
                        sane_scan::ValueType::String => {
                            DeviceOptionValue::String(CString::new(val.to_string()).unwrap())
                        }
                        _ => {
                            continue;
                        }
                    };

                    device.set_option(&opt, val).into_diagnostic()?;
                }
            }

            let params = device.start_scan().into_diagnostic()?;

            let data = device.read_to_vec().into_diagnostic()?;

            println!("{params:?}");

            let img = match params.format {
                sane_scan::Frame::Gray => DynamicImage::from(
                    image::GrayImage::from_raw(
                        params.pixels_per_line as u32,
                        params.lines as u32,
                        data,
                    )
                    .unwrap(),
                ),
                sane_scan::Frame::Rgb => DynamicImage::from(
                    image::RgbImage::from_raw(
                        params.pixels_per_line as u32,
                        params.lines as u32,
                        data,
                    )
                    .unwrap(),
                ),
                sane_scan::Frame::Red => todo!(),
                sane_scan::Frame::Green => todo!(),
                sane_scan::Frame::Blue => todo!(),
            };

            let mut jpeg_encoder = JpegEncoder::new(file);

            jpeg_encoder.encode_image(&img).into_diagnostic()?;
        }
    }

    Ok(())
}
