use std::collections::HashMap;
use std::ffi::CString;

use image::codecs::jpeg::JpegEncoder;
use image::DynamicImage;
use miette::Context;
use miette::IntoDiagnostic;
use sane_scan::DeviceOptionValue;
use sane_scan::Sane;

use crate::error::ScannrsError;

pub fn scan(
    sane: Sane,
    name: String,
    path: std::path::PathBuf,
    options: Vec<(Vec<u8>, String)>,
) -> Result<(), miette::Error> {
    let mut device = match sane
        .get_devices()
        .into_diagnostic()?
        .into_iter()
        .find_map(|d| (d.name.as_bytes() == name.as_bytes()).then(|| d.open()))
    {
        Some(device) => device
            .map_err(ScannrsError::from)
            .into_diagnostic()
            .with_context(|| format!("While trying to open a connection with scanner {}", name))?,
        None => return Err(ScannrsError::CouldNotFindScanner { name }.into()),
    };
    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .into_diagnostic()
        .with_context(|| format!("Tried to write to file at {}", path.display()))?;
    let options = options.into_iter().collect::<HashMap<_, _>>();
    for opt in device.get_options().into_diagnostic()? {
        if let Some(val) = options.get(opt.name.as_bytes()) {
            let val = match opt.type_ {
                sane_scan::ValueType::Int => {
                    DeviceOptionValue::Int(val.parse().into_diagnostic()?)
                }
                sane_scan::ValueType::String => DeviceOptionValue::String(
                    CString::new(val.to_string()).into_diagnostic().with_context(|| {
                        format!("The value given for '{}' contains a NUL (\\0) byte, which is invalid", opt.name.to_string_lossy())
                    })?,
                ),
                _ => {
                    continue;
                }
            };

            device.set_option(&opt, val).into_diagnostic()?;
        }
    }
    let params = device.start_scan().into_diagnostic()?;
    let data = device.read_to_vec().into_diagnostic()?;
    let buffer_size = data.len();
    let img = match params.format {
        sane_scan::Frame::Gray => DynamicImage::from(
            image::GrayImage::from_raw(params.pixels_per_line as u32, params.lines as u32, data)
                .ok_or(ScannrsError::InvalidImageSize {
                    width: params.pixels_per_line as u32,
                    height: params.lines as u32,
                    buffer_size,
                    pixel_size: params.depth as u32,
                })
                .into_diagnostic()?,
        ),
        sane_scan::Frame::Rgb => DynamicImage::from(
            image::RgbImage::from_raw(params.pixels_per_line as u32, params.lines as u32, data)
                .ok_or(ScannrsError::InvalidImageSize {
                    width: params.pixels_per_line as u32,
                    height: params.lines as u32,
                    buffer_size,
                    pixel_size: params.depth as u32,
                })
                .into_diagnostic()?,
        ),
        sane_scan::Frame::Red => todo!(),
        sane_scan::Frame::Green => todo!(),
        sane_scan::Frame::Blue => todo!(),
    };
    let mut jpeg_encoder = JpegEncoder::new(file);
    jpeg_encoder.encode_image(&img).into_diagnostic()?;
    Ok(())
}
