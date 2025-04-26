use std::fmt;

#[derive(Debug)]
pub enum RomError {
    IoError(std::io::Error),
    FileTooSmall,
    _InvalidFormat,
}

impl std::error::Error for RomError {}
impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomError::IoError(e) => write!(f, "I/O error: {}", e),
            RomError::FileTooSmall => write!(f, "ROM file too small to be valid."),
            RomError::_InvalidFormat => write!(f, "ROM file has an invalid format."),
        }
    }
}
