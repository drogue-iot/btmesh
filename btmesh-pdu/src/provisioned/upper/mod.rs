use crate::provisioned::upper::access::UpperAccessPDU;
use crate::provisioned::upper::control::UpperControlPDU;
use crate::provisioned::System;

pub mod access;
pub mod control;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UpperPDU<S: System> {
    Access(UpperAccessPDU<S>),
    Control(UpperControlPDU<S>),
}

impl<S: System> Clone for UpperPDU<S> {
    fn clone(&self) -> Self {
        match self {
            UpperPDU::Access(inner) => {
                let inner = inner.clone();
                UpperPDU::Access(inner)
            }
            UpperPDU::Control(inner) => {
                let inner = inner.clone();
                UpperPDU::Control(inner)
            }
        }
    }
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
