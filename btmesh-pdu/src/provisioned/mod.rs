use crate::provisioned::access::AccessMessage;
use crate::provisioned::control::ControlMessage;

pub mod access;
pub mod control;
pub mod lower;
pub mod network;
pub mod proxy;
pub mod upper;

#[cfg(not(feature = "defmt"))]
pub trait System {
    type NetworkKeyHandle: Copy;
    type ApplicationKeyHandle: Copy;

    type NetworkMetadata: Clone;
    type LowerMetadata;
    type UpperMetadata: Clone;
    type AccessMetadata;
    type ControlMetadata;
}

#[cfg(feature = "defmt")]
pub trait System: ::defmt::Format {
    type NetworkKeyHandle: Copy;
    type ApplicationKeyHandle: Copy;

    type NetworkMetadata: ::defmt::Format + Clone;
    type LowerMetadata: ::defmt::Format;
    type UpperMetadata: ::defmt::Format + Clone;
    type AccessMetadata: ::defmt::Format;
    type ControlMetadata: ::defmt::Format;
}

impl System for () {
    type NetworkKeyHandle = ();
    type ApplicationKeyHandle = ();
    type NetworkMetadata = ();
    type LowerMetadata = ();
    type UpperMetadata = ();
    type AccessMetadata = ();
    type ControlMetadata = ();
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Message<S: System> {
    Access(AccessMessage<S>),
    Control(ControlMessage<S>),
}
