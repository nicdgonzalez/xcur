use std::{error, fmt, io};

/// An error occurred while parsing the data.
#[derive(Debug)]
pub enum ParseError {
    /// An I/O-related error, such as failure to read bytes from a file
    Io {
        /// The underlying failure that caused the error
        source: io::Error,
    },
    /// Data is not in the expected format
    InvalidSignature,
    /// Expected to read more bytes than were available
    NotEnoughBytes {
        /// Number of additional bytes expected
        needed: usize,
    },
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::Io { ref source } => Some(source),
            Self::InvalidSignature | Self::NotEnoughBytes { needed: _ } => None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Io { source: _ } => "I/O error".fmt(f),
            Self::InvalidSignature => "invalid file signature (A.K.A. magic number)".fmt(f),
            Self::NotEnoughBytes { needed } => {
                write!(f, "not enough bytes (needed {needed} more bytes)")
            }
        }
    }
}
