use crate::Configuration;
use btmesh_common::{Composition, Uuid};
use rand_core::RngCore;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Hash, Debug)]
pub struct UnprovisionedConfiguration {
    pub(crate) uuid: Uuid,
}

impl UnprovisionedConfiguration {
    pub fn display(&self, composition: &Composition) {
        info!("uuid: {}", self.uuid);
    }

    pub fn new<R: RngCore>(rng: &mut R) -> Self {
        Self {
            uuid: Uuid::new_random(rng),
        }
    }
}

impl From<UnprovisionedConfiguration> for Configuration {
    fn from(inner: UnprovisionedConfiguration) -> Self {
        Self::Unprovisioned(inner)
    }
}
