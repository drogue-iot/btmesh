use crate::DriverError;
use btmesh_common::{Composition, ModelIdentifier};
use btmesh_models::foundation::configuration::AppKeyIndex;
use heapless::Vec;

#[derive(Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
pub struct Bindings<const N: usize = 8> {
    elements: Vec<ElementBindings, N>,
}

impl<const N: usize> Default for Bindings<N> {
    fn default() -> Self {
        let mut elements = Vec::new();
        elements.resize(N, ElementBindings::default()).ok();
        Self { elements }
    }
}

impl<const N: usize> Bindings<N> {
    pub fn display(&self, composition: &Composition) {
        info!("== app-key bindings ==");
        for (index, element) in composition.elements_iter().enumerate() {
            info!("elements[{}]", index);
            let element_bindings = &self.elements[index];
            for model_descriptor in element.models_iter() {
                if let Some(app_key_index) =
                    element_bindings.binding_for(&model_descriptor.model_identifier)
                {
                    info!(
                        "  {} - app_key_index: {}",
                        model_descriptor.model_identifier, app_key_index
                    );
                } else {
                    info!("  {}", model_descriptor.model_identifier);
                }
            }
        }
    }

    pub fn bind(
        &mut self,
        composition: &Composition,
        element_index: u8,
        model_identifier: ModelIdentifier,
        app_key_index: AppKeyIndex,
    ) -> Result<(), DriverError> {
        if element_index as usize >= N {
            Err(DriverError::InvalidModel)
        } else {
            let descriptor = &composition[element_index];
            if descriptor.has_model(model_identifier) {
                self.elements[element_index as usize].bind(model_identifier, app_key_index)?;
                Ok(())
            } else {
                Err(DriverError::InvalidModel)
            }
        }
    }

    pub fn unbind(
        &mut self,
        composition: &Composition,
        element_index: u8,
        model_identifier: ModelIdentifier,
        app_key_index: AppKeyIndex,
    ) -> Result<(), DriverError> {
        if element_index as usize >= N {
            Err(DriverError::InvalidModel)
        } else {
            let descriptor = &composition[element_index];
            if descriptor.has_model(model_identifier) {
                self.elements[element_index as usize].unbind(model_identifier, app_key_index)?;
                Ok(())
            } else {
                Err(DriverError::InvalidModel)
            }
        }
    }
}

#[derive(Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
pub struct ElementBindings<const N: usize = 4> {
    bindings: Vec<Option<ModelBinding>, N>,
}

impl<const N: usize> Default for ElementBindings<N> {
    fn default() -> Self {
        let mut bindings = Vec::new();
        bindings.resize(N, None).ok();
        Self { bindings }
    }
}

impl ElementBindings {
    pub fn bind(
        &mut self,
        model_identifier: ModelIdentifier,
        app_key_index: AppKeyIndex,
    ) -> Result<(), DriverError> {
        if let Some(slot) = self.bindings.iter_mut().find(|e| matches!(e, None)) {
            slot.replace(ModelBinding {
                model_identifier,
                app_key_index,
            });
            Ok(())
        } else {
            Err(DriverError::InsufficientSpace)
        }
    }

    pub fn unbind(
        &mut self,
        model_identifier: ModelIdentifier,
        app_key_index: AppKeyIndex,
    ) -> Result<(), DriverError> {
        if let Some(slot) = self.bindings.iter_mut().find(|e| {
            if let Some(slot) = e {
                slot.app_key_index == app_key_index && slot.model_identifier == model_identifier
            } else {
                false
            }
        }) {
            slot.take();
        }
        Ok(())
    }

    pub fn binding_for(&self, model_identifier: &ModelIdentifier) -> Option<AppKeyIndex> {
        self.bindings.iter().flatten().find_map(|e| {
            if e.model_identifier == *model_identifier {
                Some(e.app_key_index)
            } else {
                None
            }
        })
    }
}

#[derive(Clone, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
struct ModelBinding {
    model_identifier: ModelIdentifier,
    app_key_index: AppKeyIndex,
}

impl ModelBinding {}
