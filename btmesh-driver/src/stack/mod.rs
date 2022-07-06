use crate::stack::unprovisioned::UnprovisionedStack;
use crate::{DeviceState, ProvisionedStack, Sequence};

pub mod interface;
pub mod provisioned;
pub mod unprovisioned;

#[allow(clippy::large_enum_variant)]
pub enum Stack {
    Unprovisioned(UnprovisionedStack, u8),
    Provisioned(ProvisionedStack, Sequence),
}

impl Stack {
    pub fn device_state(&self) -> DeviceState {
        match self {
            Stack::Unprovisioned(_, _) => {
                DeviceState::Unprovisioned { uuid: todo!() }
            }
            Stack::Provisioned(_, _) => DeviceState::Provisioned,
        }
    }
}
