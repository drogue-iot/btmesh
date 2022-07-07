use crate::stack::unprovisioned::UnprovisionedStack;
use crate::{DeviceState, ProvisionedStack, Sequence};
use btmesh_common::Uuid;

pub mod interface;
pub mod provisioned;
pub mod unprovisioned;

#[allow(clippy::large_enum_variant)]
pub enum Stack {
    Unprovisioned(UnprovisionedStack, u8, Uuid),
    Provisioned(ProvisionedStack, Sequence),
}

impl Stack {
    pub fn device_state(&self) -> DeviceState {
        match self {
            Stack::Unprovisioned(_, _, uuid) => DeviceState::Unprovisioned { uuid: *uuid },
            Stack::Provisioned(_, _) => DeviceState::Provisioned,
        }
    }
}
