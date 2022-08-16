use crate::storage::provisioned::foundation::Foundation;
use crate::{Configuration, DeviceInfo, NetworkState, Secrets};
use btmesh_common::Composition;
use core::hash::{Hash, Hasher};

mod foundation;

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ProvisionedConfiguration {
    pub(crate) sequence: u32,
    pub(crate) network_state: NetworkState,
    pub(crate) secrets: Secrets,
    pub(crate) device_info: DeviceInfo,
    pub(crate) foundation: Foundation,
}

impl ProvisionedConfiguration {
    pub fn display(&self, composition: &Composition) {
        info!("========================================================================");
        info!("=  Provisioned                                                         =");
        info!("------------------------------------------------------------------------");
        info!("seq: {}", self.sequence);
        self.device_info.display();
        self.network_state.display();
        self.secrets.display();
        info!("========================================================================");
    }

    pub fn network_state(&self) -> NetworkState {
        self.network_state
    }

    pub fn secrets(&self) -> Secrets {
        self.secrets.clone()
    }

    pub fn device_info(&self) -> DeviceInfo {
        self.device_info
    }

    pub fn sequence(&self) -> u32 {
        self.sequence as u32
    }

    pub fn foundation(&self) -> &Foundation {
        &self.foundation
    }

    pub fn foundation_mut(&mut self) -> &mut Foundation {
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
        }
    }
}

impl Hash for ProvisionedConfiguration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.network_state.hash(state);
        self.secrets.hash(state);
        self.device_info.hash(state);
        // explicitly skip sequence, checked separately.
    }
}

impl From<ProvisionedConfiguration> for Configuration {
    fn from(inner: ProvisionedConfiguration) -> Self {
        Configuration::Provisioned(inner)
    }
}
