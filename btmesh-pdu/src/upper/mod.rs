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
