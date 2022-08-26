use btmesh_device::{join, BluetoothMeshModel, BluetoothMeshModelContext };
use btmesh_macro::{device, element};
use btmesh_models::generic::onoff::{
    GenericOnOffClient, GenericOnOffMessage, GenericOnOffServer, Set,
};
use core::future::Future;
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull};
use embassy_futures::{select, Either};

#[device(cid = 0x0003, pid = 0x0001, vid = 0x0001)]
pub struct Device<'d> {
    zero: ElementZero<'d>,
}

#[element(location = "left")]
struct ElementZero<'d> {
    led: MyOnOffServerHandler<'d>,
    button: MyOnOffClientHandler<'d>,
}

impl Device<'_> {
    pub fn new(led: AnyPin, button: AnyPin) -> Self {
        Self {
            zero: ElementZero::new(led, button),
        }
    }
}

impl<'d> ElementZero<'d> {
    fn new(led: AnyPin, button: AnyPin) -> Self {
        Self {
            led: MyOnOffServerHandler::new(led),
            button: MyOnOffClientHandler::new(button),
        }
    }
}

struct MyOnOffServerHandler<'d> {
    led: Output<'d, AnyPin>,
}

impl<'d> MyOnOffServerHandler<'d> {
    fn new(pin: AnyPin) -> Self {
        Self {
            led: Output::new(pin, Level::Low, OutputDrive::Standard),
        }
    }
}

impl BluetoothMeshModel<GenericOnOffServer> for MyOnOffServerHandler<'_> {
    type RunFuture<'f, C> = impl Future<Output=Result<(), ()>> + 'f
    where
        Self: 'f,
        C: BluetoothMeshModelContext<GenericOnOffServer> + 'f;

    fn run<'run, C: BluetoothMeshModelContext<GenericOnOffServer> + 'run>(
        &'run mut self,
        ctx: C,
    ) -> Self::RunFuture<'_, C> {
        async move {
            loop {
                let (message, meta) = ctx.receive().await;
                match message {
                    GenericOnOffMessage::Get => {}
                    GenericOnOffMessage::Set(val) => {}
                    GenericOnOffMessage::SetUnacknowledged(val) => {}
                    GenericOnOffMessage::Status(_) => {
                        // not applicable
                    }
                }
            }
        }
    }
}

struct MyOnOffClientHandler<'d> {
    button: Input<'d, AnyPin>,
}

impl MyOnOffClientHandler<'_> {
    fn new(button: AnyPin) -> Self {
        Self {
            button: Input::new(button, Pull::Up),
        }
    }
}

impl BluetoothMeshModel<GenericOnOffClient> for MyOnOffClientHandler<'_> {
    type RunFuture<'f, C> = impl Future<Output=Result<(), ()>> + 'f
    where
        Self: 'f,
        C: BluetoothMeshModelContext<GenericOnOffClient> + 'f;

    #[allow(clippy::await_holding_refcell_ref)]
    fn run<'run, C: BluetoothMeshModelContext<GenericOnOffClient> + 'run>(
        &'run mut self,
        ctx: C,
    ) -> Self::RunFuture<'_, C> {
        async move {
            loop {
                let button_fut = self.button.wait_for_any_edge();
                let message_fut = ctx.receive();

                match select(button_fut, message_fut).await {
                    Either::First(_) => {
                        defmt::info!("** button toggled");
                        ctx.publish(GenericOnOffMessage::SetUnacknowledged(Set {
                            on_off: if self.button.is_high() { 128 } else {0 },
                            tid: 0,
                            transition_time: None,
                            delay: None
                        })).await;
                    }
                    Either::Second(message) => {
                        defmt::info!("** message received");
                    }
                }
            }
        }
    }
}
