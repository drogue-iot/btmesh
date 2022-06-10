use crate::upper::access::UpperAccessPDU;
use crate::upper::control::UpperControlPDU;
use crate::System;

pub mod access;
pub mod control;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UpperPDU<S: System> {
    Access(UpperAccessPDU<S>),
    Control(UpperControlPDU<S>),
}

/*
impl<S: System> TryInto<AccessMessage<S>> for UpperPDU<S> {
    type Error = ParseError;

    fn try_into(self) -> Result<AccessMessage<S>, Self::Error> {
        match self {
            UpperPDU::Control(_) => Err(ParseError::InvalidPDUFormat),
            UpperPDU::Access(inner) => AccessMessage::parse(&inner),
        }
    }
}

 */
