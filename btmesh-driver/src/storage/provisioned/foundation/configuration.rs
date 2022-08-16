use btmesh_common::Ttl;

#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[derive(Clone, Debug)]
pub struct Configuration {
    beacon: bool,
    default_ttl: Ttl,
}

impl Configuration {
    pub fn beacon(&self) -> bool {
        self.beacon
    }

    pub fn beacon_mut(&mut self) -> &mut bool {
        &mut self.beacon
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
        }
    }
}
