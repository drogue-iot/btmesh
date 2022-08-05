use crate::ProvisionedStack;
use btmesh_device::SenderImpl;
use btmesh_pdu::provisioned::access::AccessMessage;

pub struct Dispatcher {
    foundation_sender: SenderImpl,
    device_sender: SenderImpl,
}

impl Dispatcher {
    pub fn new(foundation_sender: SenderImpl, device_sender: SenderImpl) -> Self {
        Self {
            foundation_sender,
            device_sender,
        }
    }

    pub async fn dispatch(&self, message: AccessMessage<ProvisionedStack>) {
        info!("dispatch not implemented yet. sorry, guv.");
    }
}
