#![allow(dead_code)]

pub mod access;
pub mod lower;
pub mod network;
pub mod proxy;
pub mod upper;

pub trait System {
    type NetworkKeyHandle: Copy;
    type ApplicationKeyHandle: Copy;

    type NetworkMetadata: Copy;
    type LowerMetadata: Copy;
    type UpperMetadata: Copy;
    type AccessMetadata: Copy;
}

impl System for () {
    type NetworkKeyHandle = ();
    type ApplicationKeyHandle = ();
    type NetworkMetadata = ();
    type LowerMetadata = ();
    type UpperMetadata = ();
    type AccessMetadata = ();
}
