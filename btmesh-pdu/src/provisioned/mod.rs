use crate::provisioned::access::AccessMessage;
use crate::provisioned::control::ControlMessage;

pub mod access;
pub mod control;
pub mod lower;
pub mod network;
pub mod proxy;
pub mod upper;

pub trait System {
    type NetworkKeyHandle: Copy;
    type ApplicationKeyHandle: Copy;

    type NetworkMetadata;
    type LowerMetadata;
    type UpperMetadata;
    type AccessMetadata;
    type ControlMetadata;
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

pub enum Message<S: System> {
    Access(AccessMessage<S>),
    Control(ControlMessage<S>),
}
