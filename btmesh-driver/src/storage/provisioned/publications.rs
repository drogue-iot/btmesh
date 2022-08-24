use crate::DriverError;
use btmesh_common::{Composition, ModelIdentifier, Ttl};
use btmesh_models::foundation::configuration::model_publication::{
    PublicationDetails, PublishAddress,
};
use btmesh_models::foundation::configuration::AppKeyIndex;
use heapless::Vec;

#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Clone, Debug, Hash)]
pub struct Publications<const N: usize = 16> {
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
            for model_identifier in element.models_iter() {
                if let Some(publication) = self.get(index as u8, *model_identifier) {
                    info!(
                        "  {} --> {} {}/{}",
                        model_identifier,
                        publication.publish_address,
                        publication.publish_ttl,
                        publication.publish_period
                    );
                }
            }
        }
    }

    pub fn get(&self, element_index: u8, model_identifier: ModelIdentifier) -> Option<Publication> {
        self.entries.iter().find_map(|e| {
            if let Some(slot) = e {
                if slot.element_index == element_index && slot.model_identifier == model_identifier
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
                        && slot.model_identifier == details.model_identifier
                } else {
                    false
                }
            }) {
                slot.take();
            }
            return Ok(());
        }

        if self.entries.iter().any(|e| {
            if let Some(slot) = e {
                slot.element_index == element_index
                    && slot.model_identifier == details.model_identifier
            } else {
                false
            }
        }) {
            return Ok(());
        }

        let descriptor = &composition[element_index];
        if descriptor.has_model(details.model_identifier) {
            if let Some(slot) = self.entries.iter_mut().find(|e| matches!(e, None)) {
                slot.replace(Publication {
                    element_index,
                    publish_address: details.publish_address,
                    app_key_index: details.app_key_index,
                    credential_flag: details.credential_flag,
                    publish_ttl: details.publish_ttl,
                    publish_period: details.publish_period,
                    publish_retransmit_count: details.publish_retransmit_count,
                    publish_retransmit_interval_steps: details.publish_retransmit_interval_steps,
                    model_identifier: details.model_identifier,
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
    pub publish_address: PublishAddress,
    pub app_key_index: AppKeyIndex,
    pub credential_flag: bool,
    pub publish_ttl: Option<Ttl>,
    pub publish_period: u8,
    pub publish_retransmit_count: u8,
    pub publish_retransmit_interval_steps: u8,
    pub model_identifier: ModelIdentifier,
}