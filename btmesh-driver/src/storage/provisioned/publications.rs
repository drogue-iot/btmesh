use crate::DriverError;
use btmesh_common::{Composition, ModelIdentifier};
use btmesh_models::foundation::configuration::model_publication::{
    PublicationDetails, PublishAddress,
};
use core::hash::Hash;
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug, Hash)]
pub struct Publications<const N: usize = 8> {
    entries: Vec<Option<Publication>, N>,
}

impl<const N: usize> Default for Publications<N> {
    fn default() -> Self {
        let mut entries = Vec::new();
        entries.resize(N, None).ok();
        Self { entries }
    }
}

impl<const N: usize> Publications<N> {
    pub fn display(&self, composition: &Composition) {
        info!("== publications ==");
        for (index, element) in composition.elements_iter().enumerate() {
            info!("elements[{}]", index);
            for model_descriptor in element.models_iter() {
                if let Some(publication) = self.get(index as u8, model_descriptor.model_identifier)
                {
                    info!(
                        "  {} --> {} {}/{}",
                        model_descriptor.model_identifier,
                        publication.details.publish_address,
                        publication.details.publish_ttl,
                        publication.details.publish_period
                    );
                }
            }
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Publication> {
        self.entries.iter_mut().flatten()
    }

    pub fn get(&self, element_index: u8, model_identifier: ModelIdentifier) -> Option<Publication> {
        self.entries.iter().find_map(|e| {
            if let Some(slot) = e {
                if slot.element_index == element_index
                    && slot.details.model_identifier == model_identifier
                {
                    Some(slot.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn set(
        &mut self,
        composition: &Composition,
        element_index: u8,
        details: PublicationDetails,
    ) -> Result<(), DriverError> {
        if let PublishAddress::Unassigned = details.publish_address {
            for slot in self.entries.iter_mut().filter(|e| {
                if let Some(slot) = e {
                    slot.element_index == element_index
                        && slot.details.model_identifier == details.model_identifier
                } else {
                    false
                }
            }) {
                slot.take();
            }
            return Ok(());
        }

        if let Some(slot) = self.entries.iter_mut().find(|e| {
            if let Some(slot) = e {
                slot.element_index == element_index
                    && slot.details.model_identifier == details.model_identifier
            } else {
                false
            }
        }) {
            slot.replace(Publication {
                element_index,
                details,
            });
            return Ok(());
        }

        let descriptor = &composition[element_index];
        if descriptor.has_model(details.model_identifier) {
            if let Some(slot) = self.entries.iter_mut().find(|e| matches!(e, None)) {
                slot.replace(Publication {
                    element_index,
                    details,
                });
                Ok(())
            } else {
                Err(DriverError::InsufficientSpace)
            }
        } else {
            Err(DriverError::InvalidModel)
        }
    }
}

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug, Hash)]
pub struct Publication {
    pub element_index: u8,
    pub details: PublicationDetails,
}
