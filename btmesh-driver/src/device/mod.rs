use btmesh_common::ModelIdentifier;
use btmesh_device::access_counted::AccessCountedHandle;
use btmesh_device::{
    BluetoothMeshDeviceContext, BluetoothMeshElementContext, BluetoothMeshModelContext,
    CompletionStatus, CompletionToken, InboundMetadata, InboundModelChannelReceiver,
    InboundPayload, InboundReceiverImpl, Model, OutboundMetadata, OutboundSenderImpl,
};
use btmesh_models::Message;
use core::future::Future;
use embassy_sync::signal::Signal;
use heapless::Vec;

pub(crate) struct DeviceContext {
    inbound: InboundReceiverImpl,
    outbound: OutboundSenderImpl,
}

impl DeviceContext {
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

    type ReceiveFuture<'f> = impl Future<Output =AccessCountedHandle<'static, InboundPayload>> + 'f
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
    type ModelContext<'m, M: Model> = ModelContext<'m, M>
        where M: 'm, Self: 'm;

    fn model_context<'m, M: Model + 'm>(
        &'m self,
        _index: usize,
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
    outbound: OutboundSenderImpl,
}

impl<M: Model> BluetoothMeshModelContext<M> for ModelContext<'_, M> {
    type ReceiveFuture<'f> = impl Future<Output = (M::Message, InboundMetadata)> + 'f
    where
        Self: 'f,
        M: 'f;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        self.inbound.recv()
        /*
        async move {
            loop {
                //let (_index, opcode, parameters, meta) = &*self.inbound.recv().await;

                //info!("**** parse {}", opcode);

                match M::parse(*opcode, parameters) {
                    Ok(Some(message)) => {
                        return (message, *meta);
                    }
                    Ok(None) => {
                        continue;
                    }
                    Err(err) => {
                        info!("problems parsing {}", err);
                    }
                }
            }
        }
         */
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
                info!("sending outbound");
                self.outbound
                    .send((
                        (self.element_index, self.model_identifier),
                        opcode,
                        parameters,
                        meta,
                        Some(CompletionToken::new(signal)),
                    ))
                    .await;
                info!("sending outbound complete");
            }

            info!("waiting completion signal");
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
