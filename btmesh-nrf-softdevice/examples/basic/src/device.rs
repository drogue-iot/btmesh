use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_macro::{device, element};
use btmesh_models::generic::onoff::{GenericOnOffClient, GenericOnOffMessage, GenericOnOffServer, Set, Status};
use core::future::Future;
use defmt::info;
use embassy_futures::{select, Either};
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull};

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
                    GenericOnOffMessage::Set(val) => {
                        let present_on_off =
                        if val.on_off == 0 {
                            self.led.set_high();
                            0
                        } else {
                            self.led.set_low();
                            1
                        };
                        ctx.send(Status {
                            present_on_off,
                            target_on_off: present_on_off,
                            remaining_time: 0
                        }.into(), meta.reply()).await;
                    }
                    GenericOnOffMessage::SetUnacknowledged(val) => {
                        if val.on_off == 0 {
                            self.led.set_high();
                        } else {
                            self.led.set_low();
                        }
                    }
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
            let mut tid = 0;
            loop {
                let button_fut = self.button.wait_for_any_edge();
                let message_fut = ctx.receive();

                match select(button_fut, message_fut).await {
                    Either::First(_) => {
                        defmt::info!("** button toggled {}", tid);
                        ctx.publish(GenericOnOffMessage::Set(Set {
                            on_off: if self.button.is_high() { 0 } else { 1 },
                            tid,
                            transition_time: None,
                            delay: None,
                        }))
                        .await
                        .ok();
                        tid += 1;
                    }
                    Either::Second(_message) => {
                        defmt::info!("** message received");
                    }
                }
            }
        }
    }
}
