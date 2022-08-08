use crate::ProvisionedStack;
use btmesh_device::InboundSenderImpl;
use btmesh_pdu::provisioned::access::AccessMessage;
use crate::stack::provisioned::system::AccessMetadata;

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

    pub async fn dispatch(&self, message: AccessMessage<ProvisionedStack>) {
        info!("dispatch not implemented yet. sorry, guv.");
    }
}
