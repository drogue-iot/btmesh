use crate::stack::provisioned::system::AccessMetadata;
use crate::{DriverError, ProvisionedStack};
use btmesh_device::InboundSenderImpl;
use btmesh_pdu::provisioned::access::AccessMessage;
use heapless::Vec;

pub struct Dispatcher {
    foundation_sender: InboundSenderImpl,
    device_sender: InboundSenderImpl,
}

impl Dispatcher {
    pub fn new(foundation_sender: InboundSenderImpl, device_sender: InboundSenderImpl) -> Self {
        Self {
            foundation_sender,
            device_sender,
        }
    }

    pub async fn dispatch(
        &self,
        message: AccessMessage<ProvisionedStack>,
    ) -> Result<(), DriverError> {
        let opcode = message.opcode();
        let parameters = message.parameters();
        let local_element_index = message.meta().local_element_index();

        let meta = message.meta().into();

        info!("opcode {}", opcode);
        info!("local_element_index {}", local_element_index);

        if let Some(local_element_index) = local_element_index {
            if local_element_index == 0 {
                self.foundation_sender
                    .send((Some(0usize), opcode, Vec::from_slice(parameters)?, meta))
                    .await;
            }
            self.device_sender
                .send((
                    Some(local_element_index as usize),
                    opcode,
                    Vec::from_slice(parameters)?,
                    meta,
                ))
                .await;
        } else {
            self.foundation_sender
                .send((None, opcode, Vec::from_slice(parameters)?, meta))
                .await;
            self.device_sender
                .send((None, opcode, Vec::from_slice(parameters)?, meta))
                .await;
        }

        Ok(())
    }
}
