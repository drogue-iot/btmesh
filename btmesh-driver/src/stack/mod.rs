use crate::stack::unprovisioned::UnprovisionedStack;
use crate::{DeviceState, ProvisionedStack, Sequence};

pub mod interface;
pub mod provisioned;
pub mod unprovisioned;

pub enum Stack {
    Unprovisioned(UnprovisionedStack, u8),
    Provisioned(ProvisionedStack, Sequence),
}

impl Stack {
    pub fn device_state(&self) -> DeviceState {
        match self {
            Stack::Unprovisioned(stack, num_elements) => {
                DeviceState::Unprovisioned { uuid: todo!() }
            }
            Stack::Provisioned(_, _) => DeviceState::Provisioned,
        }
    }
}
