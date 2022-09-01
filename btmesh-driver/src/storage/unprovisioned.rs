use crate::Configuration;
use btmesh_common::{Composition, Uuid};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Hash, Debug)]
pub struct UnprovisionedConfiguration {
    pub(crate) uuid: Uuid,
}

impl UnprovisionedConfiguration {
    pub fn display(&self, _composition: &Composition) {
        info!("========================================================================");
        info!("=  Unprovisioned                                                       =");
        info!("------------------------------------------------------------------------");
        info!("uuid: {}", self.uuid);
        info!("========================================================================");
    }

    pub fn new(uuid: Uuid) -> Self {
        Self { uuid }
    }
}

impl From<UnprovisionedConfiguration> for Configuration {
    fn from(inner: UnprovisionedConfiguration) -> Self {
        Self::Unprovisioned(inner)
    }
}
