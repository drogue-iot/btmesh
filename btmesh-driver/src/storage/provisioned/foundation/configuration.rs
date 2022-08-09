use btmesh_common::Ttl;

#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Configuration {
    default_ttl: Ttl,
}

impl Configuration {
    pub fn default_ttl(&self) -> &Ttl {
        &self.default_ttl
    }

    pub fn default_ttl_mut(&mut self) -> &mut Ttl {
        &mut self.default_ttl
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            default_ttl: Ttl::new(127),
        }
    }
}
