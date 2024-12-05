use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub(crate) enum ScannrsError {
    #[error("Could not find scanner with name: '{}'", .name)]
    CouldNotFindScanner { name: String },

    #[error("An error occured while communicating with the scanner: {}", .error)]
    Sane {
        #[from]
        error: sane_scan::Error,
    },
    #[error("The given option '{}' does not exist for scanner '{}'", .option, .name)]
    OptionNotFound { name: String, option: String },

    #[error("The given option is not formatted correctly. Please use `key=value`")]
    InvalidOption,

    #[error("The scanner gave nonsensical values, or there is a bug. It was reported: {width}x{height}pixels with a\
        bitdepth of {pixel_size} to fit into {buffer_size}. If the values make sense, please report it as a bug")]
    InvalidImageSize {
        width: u32,
        height: u32,
        buffer_size: usize,
        pixel_size: u32,
    },
}
