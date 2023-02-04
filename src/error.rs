use crate::xmlutils;

pub type Result<T, E = ArchiveError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("I/O Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Zip Error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Invalid UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Invalid XML error: {0}")]
    Xml(#[from] xmlutils::XMLError),
    #[error("Parsing of this epub failed")]
    ParsingFailure,
    #[error("An invalid ID was provided")]
    InvalidId,
    #[error("Invalid UTF-8 Path")]
    PathUtf8,
}
impl From<std::string::FromUtf8Error> for ArchiveError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Utf8(e.utf8_error())
    }
}
