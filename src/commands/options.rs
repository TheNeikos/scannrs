use miette::Context;
use miette::IntoDiagnostic;
use sane_scan::Sane;

use crate::error::ScannrsError;

pub fn options(
    sane: &Sane,
    name: String,
    command: Option<crate::cli::OptionsCommand>,
) -> Result<(), miette::Error> {
    let device = match sane
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
    match command.unwrap_or_default() {
        crate::cli::OptionsCommand::List => {
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
        crate::cli::OptionsCommand::Show { option } => {
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
                    format!("While trying to read the option '{option}' from scanner '{name}'")
                })?;

            println!("{value:?}");
        }
    }

    Ok(())
}
