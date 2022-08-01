use btmesh_macro::{device, element};
use btmesh_models::generic::onoff::{GenericOnOffClient, GenericOnOffMessage, GenericOnOffServer};
use btmesh_models::ElementModelHandler;
use core::future::Future;

struct MyOnOffServerHandler {
    led: (),
}


impl ElementModelHandler<GenericOnOffServer> for MyOnOffServerHandler {
    type HandleFuture<'f> = impl Future<Output=Result<(), ()>> + 'f
        where
    Self: 'f;

    fn handle<'f>(&'f mut self, message: GenericOnOffMessage) -> Self::HandleFuture<'f> {
        async move { Ok(()) }
    }
}

struct MyOnOffClientHandler {
    button: (),
}

impl ElementModelHandler<GenericOnOffClient> for MyOnOffClientHandler {
    type HandleFuture<'f> = impl Future<Output=Result<(), ()>> + 'f
    where
    Self: 'f;

    fn handle<'f>(&'f mut self, message: GenericOnOffMessage) -> Self::HandleFuture<'f> {
        async move { Ok(()) }
    }
}

/*
#[device]
struct Device {
    zero: ElementZero,
}
 */

#[element]
struct ElementZero {
    #[model = GenericOnOffServer]
    led: MyOnOffServerHandler,
    #[model = GenericOnOffClient]
    button: MyOnOffClientHandler,
}
