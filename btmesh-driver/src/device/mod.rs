use btmesh_common::ModelIdentifier;
use btmesh_device::access_counted::AccessCountedHandle;
use btmesh_device::{
    InboundChannel,
    BluetoothMeshDeviceContext, BluetoothMeshElementContext, BluetoothMeshModelContext,
    CompletionStatus, CompletionToken, InboundChannelReceiver,
    InboundModelPayload, InboundPayload, InboundBody, Model, OutboundChannelSender, OutboundExtra,
    OutboundMetadata, OutboundPayload, SendExtra,
};
use btmesh_models::Message;
use core::future::Future;
use embassy_sync::signal::Signal;
use heapless::Vec;

pub(crate) struct DeviceContext<'a> {
    inbound: &'a InboundChannel,
    outbound: OutboundChannelSender,
}

impl<'a> DeviceContext<'a> {
    pub fn new(inbound: &'a InboundChannel, outbound: OutboundChannelSender) -> Self {
        Self { inbound, outbound }
    }
}

impl<'a> BluetoothMeshDeviceContext for DeviceContext<'a> {
    type ElementContext = ElementContext<'a>;

    fn element_context(
        &self,
        element_index: usize,
    ) -> Self::ElementContext {
        ElementContext {
            element_index,
            inbound: self.inbound,
            outbound: self.outbound.clone(),
        }
    }
}

pub(crate) struct ElementContext<'a> {
    element_index: usize,
    inbound: &'a InboundChannel,
    outbound: OutboundChannelSender,
}

impl<'a> BluetoothMeshElementContext for ElementContext<'a> {
    type ModelContext<M: Model> = ModelContext<'a>;

    fn model_context<M: Model>(&self) -> Self::ModelContext<M> {
        ModelContext {
            element_index: self.element_index,
            model_identifier: M::IDENTIFIER,
            inbound: self.inbound.subscriber().unwrap(),
            outbound: self.outbound.clone(),
        }
    }
}

pub(crate) struct ModelContext<'a> {
    element_index: usize,
    model_identifier: ModelIdentifier,
    inbound: InboundChannelReceiver<'a>,
    outbound: OutboundChannelSender,
}

impl<'a, M: Model> BluetoothMeshModelContext<M> for ModelContext<'a> {
    type ReceiveFuture<'f> = impl Future<Output = InboundModelPayload<M::Message>> + 'f
    where
        Self: 'f,
        M: 'f;

    fn receive(&mut self) -> Self::ReceiveFuture<'_> {
        async move {
            loop {
                let message = self.inbound.next_message_pure().await;
                if message.element_index == self.element_index {
                    match &message.body {
                        InboundBody::Message(message) => {
                            if let Ok(Some(model_message)) = M::parse(&message.opcode, &message.parameters) {
                                return InboundModelPayload::Message(model_message, message.meta);
                            }
                        }
                        InboundBody::Control(control) => {
                            return InboundModelPayload::Control(*control);
                        }
                    }
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
        signal: &'static Signal<CompletionStatus>,
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
