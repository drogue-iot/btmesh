use crate::storage::provisioned::bindings::Bindings;
use crate::storage::provisioned::foundation::Foundation;
use crate::{Configuration, DeviceInfo, NetworkState, Secrets};
use btmesh_common::Composition;
use core::hash::{Hash, Hasher};

mod bindings;
mod foundation;

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ProvisionedConfiguration {
    sequence: u32,
    network_state: NetworkState,
    secrets: Secrets,
    device_info: DeviceInfo,
    bindings: Bindings,
    foundation: Foundation,
}

impl ProvisionedConfiguration {
    pub(crate) fn new(
        sequence: u32,
        network_state: NetworkState,
        secrets: Secrets,
        device_info: DeviceInfo,
        foundation: Foundation,
    ) -> Self {
        Self {
            sequence,
            network_state,
            secrets,
            device_info,
            foundation,
            bindings: Default::default(),
        }
    }

    pub fn display(&self, composition: &Composition) {
        info!("========================================================================");
        info!("=  Provisioned                                                         =");
        info!("------------------------------------------------------------------------");
        info!("seq: {}", self.sequence);
        self.device_info.display();
        self.network_state.display();
        self.secrets.display();
        self.bindings.display(composition);
        info!("========================================================================");
    }

    pub(crate) fn network_state(&self) -> &NetworkState {
        &self.network_state
    }

    pub(crate) fn secrets(&self) -> &Secrets {
        &self.secrets
    }

    pub(crate) fn secrets_mut(&mut self) -> &mut Secrets {
        &mut self.secrets
    }

    pub(crate) fn device_info(&self) -> &DeviceInfo {
        &self.device_info
    }

    pub(crate) fn bindings(&self) -> &Bindings {
        &self.bindings
    }

    pub(crate) fn bindings_mut(&mut self) -> &mut Bindings {
        &mut self.bindings
    }

    pub(crate) fn sequence(&self) -> u32 {
        self.sequence as u32
    }

    pub(crate) fn sequence_mut(&mut self) -> &mut u32 {
        &mut self.sequence
    }

    pub(crate) fn foundation(&self) -> &Foundation {
        &self.foundation
    }

    pub(crate) fn foundation_mut(&mut self) -> &mut Foundation {
        &mut self.foundation
    }
}

impl From<(DeviceInfo, Secrets, NetworkState)> for ProvisionedConfiguration {
    fn from(config: (DeviceInfo, Secrets, NetworkState)) -> Self {
        Self {
            sequence: 800,
            network_state: config.2,
            secrets: config.1,
            device_info: config.0,
            foundation: Default::default(),
            bindings: Default::default(),
        }
    }
}

impl Hash for ProvisionedConfiguration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.network_state.hash(state);
        self.secrets.hash(state);
        self.device_info.hash(state);
        self.bindings.hash(state);
        // explicitly skip sequence, checked separately.
    }
}

impl From<ProvisionedConfiguration> for Configuration {
    fn from(inner: ProvisionedConfiguration) -> Self {
        Configuration::Provisioned(inner)
    }
}
