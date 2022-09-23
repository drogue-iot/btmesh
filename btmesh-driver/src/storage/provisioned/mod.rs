use crate::storage::provisioned::bindings::Bindings;
use crate::storage::provisioned::foundation::Foundation;
use crate::storage::provisioned::publications::Publications;
use crate::storage::provisioned::subscriptions::Subscriptions;
use crate::{Configuration, DeviceInfo, NetworkState, Secrets};
use btmesh_common::{Composition, IvIndex};
use core::hash::{Hash, Hasher};

pub(crate) mod bindings;
pub(crate) mod foundation;
pub(crate) mod publications;
pub(crate) mod subscriptions;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
struct UnhashedU32(u32);

#[cfg(feature = "defmt")]
impl ::defmt::Format for UnhashedU32 {
    fn format(&self, fmt: ::defmt::Formatter) {
        ::defmt::write!(fmt, "{}", self.0);
    }
}

impl Hash for UnhashedU32 {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug, Hash)]
pub struct ProvisionedConfiguration {
    sequence: UnhashedU32,
    network_state: NetworkState,
    secrets: Secrets,
    device_info: DeviceInfo,
    bindings: Bindings,
    subscriptions: Subscriptions,
    publications: Publications,
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
            sequence: UnhashedU32(sequence),
            network_state,
            secrets,
            device_info,
            foundation,
            bindings: Default::default(),
            subscriptions: Default::default(),
            publications: Default::default(),
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
        self.subscriptions.display(composition);
        self.publications.display(composition);
        self.foundation.display();
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

    pub fn bindings_mut(&mut self) -> &mut Bindings {
        &mut self.bindings
    }

    pub(crate) fn subscriptions(&self) -> &Subscriptions {
        &self.subscriptions
    }

    pub(crate) fn subscriptions_mut(&mut self) -> &mut Subscriptions {
        &mut self.subscriptions
    }

    pub(crate) fn publications(&self) -> &Publications {
        &self.publications
    }

    pub fn publications_mut(&mut self) -> &mut Publications {
        &mut self.publications
    }

    pub(crate) fn sequence(&self) -> u32 {
        self.sequence.0 as u32
    }

    pub(crate) fn sequence_mut(&mut self) -> &mut u32 {
        &mut self.sequence.0
    }

    pub(crate) fn iv_index(&self) -> IvIndex {
        self.network_state.iv_index().transmission_iv_index()
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
            sequence: UnhashedU32(800),
            network_state: config.2,
            secrets: config.1,
            device_info: config.0,
            foundation: Default::default(),
            bindings: Default::default(),
            subscriptions: Default::default(),
            publications: Default::default(),
        }
    }
}

/*
impl Hash for ProvisionedConfiguration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.network_state.hash(state);
        self.secrets.hash(state);
        self.device_info.hash(state);
        self.bindings.hash(state);
        self.subscriptions.hash(state);
        // explicitly skip sequence, checked separately.
    }
}
 */

impl From<ProvisionedConfiguration> for Configuration {
    fn from(inner: ProvisionedConfiguration) -> Self {
        Configuration::Provisioned(inner)
    }
}
