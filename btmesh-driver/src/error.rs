use core::array::TryFromSliceError;

use crate::interface::NetworkError;
use crate::storage::StorageError;
use btmesh_common::address::InvalidAddress;
use btmesh_common::mic::InvalidLength;
use btmesh_common::{InsufficientBuffer, ParseError, SeqRolloverError};
use btmesh_pdu::provisioned::lower::InvalidBlock;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DriverError {
    InvalidState,
    InvalidFormat,
    InvalidKeyLength,
    CryptoError,
    InvalidAddress,
    InsufficientSpace,
    InvalidKeyHandle,
    InvalidNetKeyIndex,
    InvalidAppKeyIndex,
    NetKeyIndexAlreadyStored,
    AppKeyIndexAlreadyStored,
    InvalidPDU,
    IncompleteTransaction,
    Parse(ParseError),
    Network(NetworkError),
    SeqRollover,
    Storage(StorageError),
}

impl From<StorageError> for DriverError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<NetworkError> for DriverError {
    fn from(err: NetworkError) -> Self {
        Self::Network(err)
    }
}

impl From<InvalidLength> for DriverError {
    fn from(_: InvalidLength) -> Self {
        Self::CryptoError
    }
}

impl From<SeqRolloverError> for DriverError {
    fn from(_: SeqRolloverError) -> Self {
        Self::SeqRollover
    }
}

impl From<InsufficientBuffer> for DriverError {
    fn from(_: InsufficientBuffer) -> Self {
        Self::InsufficientSpace
    }
}

impl From<ParseError> for DriverError {
    fn from(inner: ParseError) -> Self {
        Self::Parse(inner)
    }
}

impl From<InvalidAddress> for DriverError {
    fn from(_: InvalidAddress) -> Self {
        Self::InvalidAddress
    }
}

impl From<InvalidBlock> for DriverError {
    fn from(_: InvalidBlock) -> Self {
        Self::InvalidState
    }
}

impl From<cmac::crypto_mac::InvalidKeyLength> for DriverError {
    fn from(e: cmac::crypto_mac::InvalidKeyLength) -> Self {
        e.into()
    }
}

impl From<TryFromSliceError> for DriverError {
    fn from(_: TryFromSliceError) -> Self {
        Self::InvalidKeyLength
    }
}

impl From<()> for DriverError {
    fn from(_: ()) -> Self {
        Self::InsufficientSpace
    }
}
