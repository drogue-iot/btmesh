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

impl<S: System> UpperPDU<S> {
    pub fn meta(&self) -> &S::UpperMetadata {
        match self {
            UpperPDU::Access(pdu) => pdu.meta(),
            UpperPDU::Control(pdu) => pdu.meta(),
        }
    }

    pub fn meta_mut(&mut self) -> &mut S::UpperMetadata {
        match self {
            UpperPDU::Access(pdu) => pdu.meta_mut(),
            UpperPDU::Control(pdu) => pdu.meta_mut(),
        }
    }
}
