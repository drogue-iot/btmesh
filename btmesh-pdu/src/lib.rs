pub mod access;
pub mod lower;
pub mod network;
pub mod proxy;
pub mod upper;

pub trait System {
    type NetworkKeyHandle: Copy;
    type ApplicationKeyHandle: Copy;

    type NetworkMetadata: Default + Copy + From<Self::LowerMetadata>;
    type LowerMetadata: Default + Copy + From<Self::NetworkMetadata> + From<Self::UpperMetadata>;
    type UpperMetadata: Default + Copy + From<Self::LowerMetadata> + From<Self::AccessMetadata>;
    type AccessMetadata: Default + Copy + From<Self::UpperMetadata>;
}

impl System for () {
    type NetworkKeyHandle = ();
    type ApplicationKeyHandle = ();
    type NetworkMetadata = ();
    type LowerMetadata = ();
    type UpperMetadata = ();
    type AccessMetadata = ();
}
