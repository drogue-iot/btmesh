use btmesh_device::{BluetoothMeshModel, BluetoothMeshModelContext};
use btmesh_macro::{device, element};
use btmesh_models::generic::onoff::{GenericOnOffClient, GenericOnOffMessage, GenericOnOffServer};
use core::cell::RefCell;
use core::future::Future;
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
        &'run self,
        ctx: C,
    ) -> Self::RunFuture<'_, C> {
        async move {
            loop {
                //Timer::after(Duration::from_secs(1)).await;
                //defmt::info!("server run loop");
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
    button: RefCell<Input<'d, AnyPin>>,
}

impl MyOnOffClientHandler<'_> {
    fn new(button: AnyPin) -> Self {
        Self {
            button: RefCell::new(Input::new(button, Pull::Up)),
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
        &'run self,
        ctx: C,
    ) -> Self::RunFuture<'_, C> {
        async move {
            loop {
                self.button.borrow_mut().wait_for_falling_edge().await;
                defmt::info!("** button pushed");
            }
        }
    }
}
