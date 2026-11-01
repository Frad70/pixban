use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Font(ParseError),
    UnknownPalette(String),
    EmptyText,
    MultiLine,
    TextTooLong { chars: usize, max: usize },
    CanvasTooLarge { width: u64, height: u64 },
    OutputTooLarge { width: u64, height: u64 },
    InvalidScale { scale: u32, max: u32 },
    InvalidShadowOffset { dx: i32, dy: i32, max: i32 },
    Encode(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Font(e) => write!(f, "font: {e}"),
            Error::UnknownPalette(name) => write!(f, "unknown palette '{name}'"),
            Error::EmptyText => write!(f, "text is empty"),
            Error::MultiLine => write!(f, "multi-line text is not supported yet"),
            Error::TextTooLong { chars, max } => {
                write!(f, "text has {chars} characters, the limit is {max}")
            }
            Error::CanvasTooLarge { width, height } => write!(
                f,
                "canvas of {width}x{height} cells exceeds the limit of {}x{} cells",
                crate::limits::MAX_GRID_SIDE,
                crate::limits::MAX_GRID_SIDE
            ),
            Error::OutputTooLarge { width, height } => write!(
                f,
                "output of {width}x{height} pixels exceeds the limit of {} pixels per side and {} in total; use a smaller scale or shorter text",
                crate::limits::MAX_OUTPUT_SIDE,
                crate::limits::MAX_OUTPUT_PIXELS
            ),
            Error::InvalidScale { scale, max } => {
                write!(f, "scale {scale} is out of range, use an integer from 1 to {max}")
            }
            Error::InvalidShadowOffset { dx, dy, max } => write!(
                f,
                "shadow offset {dx},{dy} is out of range, each value must be between -{max} and {max}"
            ),
            Error::Encode(msg) => write!(f, "encoding failed: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Font(e)
    }
}
