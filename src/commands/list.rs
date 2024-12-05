use miette::IntoDiagnostic;
use sane_scan::Sane;

pub fn list(sane: Sane) -> Result<(), miette::Error> {
    for device in sane.get_devices().into_diagnostic()? {
        println!("{device:?}");
    }

    Ok(())
}
