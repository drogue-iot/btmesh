use crate::Configuration;
use btmesh_common::Uuid;
use rand_core::RngCore;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Hash)]
pub struct UnprovisionedConfiguration {
    pub(crate) uuid: Uuid,
}

impl UnprovisionedConfiguration {
    pub fn new<R: RngCore>(rng: &mut R) -> Self {
        let mut uuid = [0; 16];
        rng.fill_bytes(&mut uuid);

        Self {
            uuid: Uuid::new(uuid),
        }
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

impl From<UnprovisionedConfiguration> for Configuration {
    fn from(inner: UnprovisionedConfiguration) -> Self {
        Self::Unprovisioned(inner)
    }
}
