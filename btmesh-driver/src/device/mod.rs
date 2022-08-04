use btmesh_device::{BluetoothMeshDeviceContext, BluetoothMeshElementContext, BluetoothMeshModelContext, ChannelImpl, Model, Opcode, ReceivePayload, ReceiverImpl};
use core::future::Future;
use embassy::blocking_mutex::raw::NoopRawMutex;
use embassy::channel::{Channel, Receiver, Sender};

pub(crate) struct DeviceContext {
    receiver: ReceiverImpl,
}

impl<'ch> DeviceContext {
    pub fn new(receiver: ReceiverImpl) -> Self {
        Self { receiver }
    }
}

impl BluetoothMeshDeviceContext for DeviceContext {
    type ElementContext = ElementContext;

    fn element_context(&self, index: usize, inbound: ChannelImpl) -> Self::ElementContext {
        ElementContext {
            inbound,
        }
    }

    type ReceiveFuture<'f> = impl Future<Output = ReceivePayload> + 'f
        where
            Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        self.receiver.recv()
    }
}

pub(crate) struct ElementContext {
    inbound: ChannelImpl,
}

impl BluetoothMeshElementContext for ElementContext {
    type ModelContext<M:Model> = ModelContext;

    fn model_context<M:Model>(&self, index: usize, inbound: ChannelImpl) -> Self::ModelContext<M> {
        ModelContext {
            inbound,
        }
    }

    type ReceiveFuture<'f> = impl Future<Output = ReceivePayload> + 'f
    where
    Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        self.inbound.recv()
    }
}

pub(crate) struct ModelContext {
    inbound: ChannelImpl
}

impl<M:Model> BluetoothMeshModelContext<M> for ModelContext {

    type ReceiveFuture<'f> = impl Future<Output = M::Message<'f>> + 'f
    where
    Self: 'f,
    M: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        async move {
            loop {
                let (opcode, parameters) = self.inbound.recv().await;

                if let Ok(Some(message)) = M::parse(opcode, &*parameters) {
                    return message
                }
            }
        }
    }

}
