use btmesh_common::ModelIdentifier;
use btmesh_device::{BluetoothMeshDeviceContext, BluetoothMeshElementContext, BluetoothMeshModelContext, CompletionStatus, CompletionToken, InboundMetadata, InboundPayload, InboundReceiverImpl, Model, OutboundMetadata, OutboundSenderImpl};
use btmesh_models::Message;
use core::future::Future;
use embassy_util::channel::signal::Signal;
use heapless::Vec;

pub(crate) struct DeviceContext {
    inbound: InboundReceiverImpl,
    outbound: OutboundSenderImpl,
}

impl<'ch> DeviceContext {
    pub fn new(inbound: InboundReceiverImpl, outbound: OutboundSenderImpl) -> Self {
        Self { inbound, outbound }
    }
}

impl BluetoothMeshDeviceContext for DeviceContext {
    type ElementContext = ElementContext;

    fn element_context(
        &self,
        element_index: usize,
        inbound: InboundReceiverImpl,
    ) -> Self::ElementContext {
        ElementContext {
            element_index,
            inbound,
            outbound: self.outbound.clone(),
        }
    }

    type ReceiveFuture<'f> = impl Future<Output =InboundPayload> + 'f
        where
            Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        self.inbound.recv()
    }
}

pub(crate) struct ElementContext {
    element_index: usize,
    inbound: InboundReceiverImpl,
    outbound: OutboundSenderImpl,
}

impl BluetoothMeshElementContext for ElementContext {
    type ModelContext<M: Model> = ModelContext;

    fn model_context<M: Model>(
        &self,
        _index: usize,
        inbound: InboundReceiverImpl,
    ) -> Self::ModelContext<M> {
        ModelContext {
            element_index: self.element_index,
            model_identifier: M::IDENTIFIER,
            inbound,
            outbound: self.outbound.clone(),
        }
    }

    type ReceiveFuture<'f> = impl Future<Output =InboundPayload> + 'f
    where
    Self: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        self.inbound.recv()
    }
}

pub(crate) struct ModelContext {
    element_index: usize,
    model_identifier: ModelIdentifier,
    inbound: InboundReceiverImpl,
    outbound: OutboundSenderImpl,
}

impl<M: Model> BluetoothMeshModelContext<M> for ModelContext {
    type ReceiveFuture<'f> = impl Future<Output = (M::Message, InboundMetadata)> + 'f
    where
        Self: 'f,
        M: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        async move {
            loop {
                let (_index, opcode, parameters, meta) = self.inbound.recv().await;

                if let Ok(Some(message)) = M::parse(opcode, &*parameters) {
                    return (message, meta);
                }
            }
        }
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
                    .send((
                        (self.element_index, self.model_identifier),
                        opcode,
                        parameters,
                        meta,
                        None,
                    ))
                    .await
            }

            Ok(())
        }
    }

    type SendWithCompletionFuture<'f> = impl Future<Output = CompletionStatus> + 'f
    where
    Self: 'f,
    M: 'f;

    fn send_with_completion(&self, message: M::Message, meta: OutboundMetadata, signal: &'static Signal<CompletionStatus>) -> Self::SendWithCompletionFuture<'_>{
        async move {
            let opcode = message.opcode();
            let mut parameters = Vec::new();
            if message.emit_parameters(&mut parameters).is_ok() {
                self.outbound
                    .send((
                        (self.element_index, self.model_identifier),
                        opcode,
                        parameters,
                        meta,
                        Some(CompletionToken::new(signal)),
                    ))
                    .await
            }

            signal.wait().await
        }
    }

    type PublishFuture<'f> = impl Future<Output = Result<(), ()>> + 'f
    where
        Self: 'f,
        M: 'f;

    fn publish(&self, _message: M::Message) -> Self::PublishFuture<'_> {
        async move { todo!() }
    }
}
