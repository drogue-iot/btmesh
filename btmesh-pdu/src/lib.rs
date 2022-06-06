use serde::{Deserialize, Serialize};

pub mod access;
pub mod lower;
pub mod network;
pub mod proxy;
pub mod upper;

pub trait System {
    type NetworkKeyHandle: Copy;
    type ApplicationKeyHandle: Copy;
}
