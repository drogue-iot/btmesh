use btmesh_common::{InsufficientBuffer, ParseError};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BearerError {
    InvalidLink,
    InvalidTransaction,
    TransmissionFailure,
    InsufficientResources,
    ParseError(ParseError),
    Unspecified,
}

impl From<()> for BearerError {
    fn from(_: ()) -> Self {
        Self::ParseError(ParseError::InsufficientBuffer)
    }
}

impl From<ParseError> for BearerError {
    fn from(e: ParseError) -> Self {
        Self::ParseError(e)
    }
}

impl From<InsufficientBuffer> for BearerError {
    fn from(_: InsufficientBuffer) -> Self {
        BearerError::InsufficientResources
    }
}

// For heapless Vec::push
impl From<u8> for BearerError {
    fn from(_: u8) -> Self {
        BearerError::InsufficientResources
    }
}
