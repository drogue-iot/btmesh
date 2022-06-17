#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

use crate::access::AccessMessage;
use crate::control::ControlMessage;

pub mod access;
pub mod lower;
pub mod network;
pub mod proxy;
pub mod upper;
pub mod control;

pub enum Message<S:System> {
    Access(AccessMessage<S>),
    Control(ControlMessage<S>),
}

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
