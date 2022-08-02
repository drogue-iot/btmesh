use btmesh_macro::{device, element};
use btmesh_models::generic::onoff::{GenericOnOffClient, GenericOnOffMessage, GenericOnOffServer};
use btmesh_models::ElementModelHandler;
use core::future::Future;


#[device(cid=0x0003, pid=0x0001, vid=0x0001)]
pub struct Device {
    zero: ElementZero,
}

impl Device {
    pub fn new() -> Self {
        Self {
            zero: ElementZero::new()
        }
    }
}

#[element(location="unknown")]
struct ElementZero {
    led: MyOnOffServerHandler,
    button: MyOnOffClientHandler,
}

impl ElementZero {
    fn new() -> Self {
        Self {
            led: MyOnOffServerHandler::new(),
            button: MyOnOffClientHandler::new(),
        }
    }
}

struct MyOnOffServerHandler {
    led: (),
}

impl MyOnOffServerHandler {
    fn new() -> Self {
        Self {
            led: ()
        }
    }
}


impl ElementModelHandler<GenericOnOffServer> for MyOnOffServerHandler {
    type HandleFuture<'f> = impl Future<Output=Result<(), ()>> + 'f
        where
    Self: 'f;

    fn handle(&mut self, _message: GenericOnOffMessage) -> Self::HandleFuture<'_> {
        async move { Ok(()) }
    }
}

struct MyOnOffClientHandler {
    button: (),
}

impl MyOnOffClientHandler {
    fn new() -> Self {
        Self {
            button: ()
        }
    }
}

impl ElementModelHandler<GenericOnOffClient> for MyOnOffClientHandler {
    type HandleFuture<'f> = impl Future<Output=Result<(), ()>> + 'f
    where
    Self: 'f;

    fn handle(&mut self, _message: GenericOnOffMessage) -> Self::HandleFuture<'_> {
        async move { Ok(()) }
    }
}
