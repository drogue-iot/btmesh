use btmesh_common::ModelIdentifier;
use btmesh_device::access_counted::AccessCountedHandle;
use btmesh_device::{
    BluetoothMeshDeviceContext, BluetoothMeshElementContext, BluetoothMeshModelContext,
    CompletionStatus, CompletionToken, InboundChannelReceiver, InboundModelChannelReceiver,
    InboundModelPayload, InboundPayload, Model, OutboundChannelSender, OutboundExtra,
    OutboundMetadata, OutboundPayload, SendExtra,
};
use btmesh_models::Message;
use core::future::Future;
//use btmesh_device::Signal;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use heapless::Vec;

pub(crate) struct DeviceContext {
    inbound: InboundChannelReceiver,
    outbound: OutboundChannelSender,
}

impl DeviceContext {
    pub fn new(inbound: InboundChannelReceiver, outbound: OutboundChannelSender) -> Self {
        Self { inbound, outbound }
    }
}

impl BluetoothMeshDeviceContext for DeviceContext {
    type ElementContext = ElementContext;

    fn element_context(
        &self,
        element_index: usize,
        inbound: InboundChannelReceiver,
    ) -> Self::ElementContext {
        ElementContext {
            element_index,
            inbound,
            outbound: self.outbound.clone(),
        }
    }

    type ReceiveFuture<'f> = impl Future<Output =AccessCountedHandle<'static, InboundPayload>> + 'f
        where
            Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        self.inbound.recv()
    }
}

pub(crate) struct ElementContext {
    element_index: usize,
    inbound: InboundChannelReceiver,
    outbound: OutboundChannelSender,
}

impl BluetoothMeshElementContext for ElementContext {
    type ModelContext<'m, M: Model> = ModelContext<'m, M>
        where M: 'm, Self: 'm;

    fn model_context<'m, M: Model + 'm>(
        &'m self,
        inbound: InboundModelChannelReceiver<'m, M::Message>,
    ) -> Self::ModelContext<'m, M> {
        ModelContext {
            element_index: self.element_index,
            model_identifier: M::IDENTIFIER,
            inbound,
            outbound: self.outbound.clone(),
        }
    }

    type ReceiveFuture<'f> = impl Future<Output =AccessCountedHandle<'static, InboundPayload>> + 'f
    where
    Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        self.inbound.recv()
    }
}

pub(crate) struct ModelContext<'m, M: Model> {
    element_index: usize,
    model_identifier: ModelIdentifier,
    inbound: InboundModelChannelReceiver<'m, M::Message>,
    outbound: OutboundChannelSender,
}

impl<M: Model> BluetoothMeshModelContext<M> for ModelContext<'_, M> {
    type ReceiveFuture<'f> = impl Future<Output = InboundModelPayload<M::Message>> + 'f
    where
        Self: 'f,
        M: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        self.inbound.recv()
    }

    type SendFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        M: 'f;

    fn send(&self, message: M::Message, meta: OutboundMetadata) -> Self::SendFuture<'_> {
        async move {
            let opcode = message.opcode();
            let mut parameters = Vec::new();
            if message.emit_parameters(&mut parameters).is_ok() {
                self.outbound
                    .send(OutboundPayload {
                        element_index: self.element_index,
                        model_identifer: self.model_identifier,
                        opcode,
                        parameters,
                        extra: SendExtra {
                            meta,
                            completion_token: None,
                        }
                        .into(),
                    })
                    .await;
            }

            Ok(())
        }
    }

    type SendWithCompletionFuture<'f> = impl Future<Output = CompletionStatus> + 'f
    where
    Self: 'f,
    M: 'f;

    fn send_with_completion(
        &self,
        message: M::Message,
        meta: OutboundMetadata,
        signal: &'static Signal<CriticalSectionRawMutex, CompletionStatus>,
    ) -> Self::SendWithCompletionFuture<'_> {
        async move {
            let opcode = message.opcode();
            let mut parameters = Vec::new();
            if message.emit_parameters(&mut parameters).is_ok() {
                self.outbound
                    .send(OutboundPayload {
                        element_index: self.element_index,
                        model_identifer: self.model_identifier,
                        opcode,
                        parameters,
                        extra: SendExtra {
                            meta,
                            completion_token: Some(CompletionToken::new(signal)),
                        }
                        .into(),
                    })
                    .await;
            }

            signal.wait().await
        }
    }

    type PublishFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        M: 'f;

    fn publish(&self, message: M::Message) -> Self::PublishFuture<'_> {
        async move {
            let opcode = message.opcode();
            let mut parameters = Vec::new();
            if message.emit_parameters(&mut parameters).is_ok() {
                self.outbound
                    .send(OutboundPayload {
                        element_index: self.element_index,
                        model_identifer: self.model_identifier,
                        opcode,
                        parameters,
                        extra: OutboundExtra::Publish,
                    })
                    .await;
            }

            Ok(())
        }
    }
}
