use btmesh_common::Ttl;
use btmesh_models::foundation::configuration::relay::RelayConfig;

#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[derive(Clone, Hash, Debug)]
pub struct Configuration {
    beacon: bool,
    relay: RelayConfig,
    default_ttl: Ttl,
}

impl Configuration {
    pub fn display(&self) {
        info!("  beacon: {}", self.beacon);
        info!("  relay: {}", self.relay);
        info!("  default_ttl: {}", self.default_ttl);
    }

    pub fn beacon(&self) -> bool {
        self.beacon
    }

    pub fn beacon_mut(&mut self) -> &mut bool {
        &mut self.beacon
    }

    pub fn relay(&self) -> &RelayConfig {
        &self.relay
    }

    pub fn relay_mut(&mut self) -> &mut RelayConfig {
        &mut self.relay
    }

    pub fn default_ttl(&self) -> Ttl {
        self.default_ttl
    }

    pub fn default_ttl_mut(&mut self) -> &mut Ttl {
        &mut self.default_ttl
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            beacon: true,
            default_ttl: Ttl::new(127),
            #[cfg(feature = "relay")]
            relay: Default::default(),
            #[cfg(not(feature = "relay"))]
            relay: RelayConfig::not_supported(),
        }
    }
}
