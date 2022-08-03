use btmesh_macro::{device, element};
use btmesh_models::generic::onoff::{GenericOnOffClient, GenericOnOffMessage, GenericOnOffServer};
use btmesh_models::ElementModelHandler;
use core::future::{pending, Future};
use core::pin::Pin;
use core::task::{Context, Poll};
use embassy::time::{Duration, Timer};
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pull};

#[device(cid = 0x0003, pid = 0x0001, vid = 0x0001)]
pub struct Device<'d> {
    zero: ElementZero<'d>,
}

impl Device<'_> {
    pub fn new(led: AnyPin, button: AnyPin) -> Self {
        Self {
            zero: ElementZero::new(led, button),
        }
    }
}

#[element(location = "unknown")]
struct ElementZero<'d> {
    led: MyOnOffServerHandler<'d>,
    button: MyOnOffClientHandler<'d>,
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

impl ElementModelHandler<GenericOnOffServer> for MyOnOffServerHandler<'_> {
    type RunFuture<'f> = impl Future<Output=Result<(), ()>> + 'f
    where
    Self: 'f;

    fn run(&self) -> Self::RunFuture<'_> {
        async move {
            loop {
                Timer::after(Duration::from_secs(1)).await;
                defmt::info!("server run loop");
            }
        }
    }

    type HandleFuture<'f> = impl Future<Output=Result<(), ()>> + 'f
        where
    Self: 'f;

    fn handle(&self, _message: GenericOnOffMessage) -> Self::HandleFuture<'_> {
        async move { Ok(()) }
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

impl ElementModelHandler<GenericOnOffClient> for MyOnOffClientHandler<'_> {
    type RunFuture<'f> = impl Future<Output=Result<(), ()>> + 'f
    where
    Self: 'f;

    fn run(&self) -> Self::RunFuture<'_> {
        async move {
            loop {
                Timer::after(Duration::from_secs(2)).await;
                defmt::info!("client run loop");
            }
        }
    }

    type HandleFuture<'f> = impl Future<Output=Result<(), ()>> + 'f
    where
    Self: 'f;

    fn handle(&self, _message: GenericOnOffMessage) -> Self::HandleFuture<'_> {
        async move { Ok(()) }
    }
}
