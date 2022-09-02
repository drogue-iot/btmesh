use crate::interface::advertising::AdvertisingBearerNetworkInterface;
use crate::interface::gatt::GattBearerNetworkInterface;
use crate::{DeviceState, Watchdog};
use btmesh_bearer::beacon::Beacon;
use btmesh_bearer::{AdvertisingBearer, BearerError, GattBearer};
use btmesh_device::join;
use btmesh_pdu::provisioning::generic::Reason;
use btmesh_pdu::PDU;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use embassy_futures::select::{select, Either};

pub mod advertising;
pub mod gatt;

/// A possibly plurality of network interfaces covering one or more bearers.
///
/// Implementations should include whatever input and output buffering that
/// makes sense for their underlying bearers.
pub trait NetworkInterfaces {
    type RunFuture<'m>: Future<Output = Result<(), NetworkError>> + 'm
    where
        Self: 'm;

    /// Run the network interfaces, stopping when the future is dropped.
    fn run(&self) -> Self::RunFuture<'_>;

    type ReceiveFuture<'m>: Future<Output = Result<PDU, NetworkError>> + 'm
    where
        Self: 'm;

    /// Receive data from any of the network interfaces.
    fn receive<'m>(
        &'m self,
        state: &'m DeviceState,
        watchdog: &'m Watchdog,
    ) -> Self::ReceiveFuture<'m>;

    type TransmitFuture<'m>: Future<Output = Result<(), NetworkError>> + 'm
    where
        Self: 'm;

    /// Transmit data on all of the network interfaces.
    fn transmit<'m>(&'m self, pdu: &'m PDU, is_retransmit: bool) -> Self::TransmitFuture<'m>;

    type BeaconFuture<'m>: Future<Output = Result<(), NetworkError>> + 'm
    where
        Self: 'm;

    /// Perform beaconing on all of the network interfaces.
    fn beacon(&self, beacon: Beacon) -> Self::BeaconFuture<'_>;

    type CloseLinkFuture<'m>: Future<Output = Result<(), NetworkError>> + 'm
    where
        Self: 'm;

    fn close_link(&self, reason: Reason) -> Self::CloseLinkFuture<'_>;

    fn reset(&self);
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum NetworkError {
    InvalidLink,
    InvalidTransaction,
    Unspecified,
    Bearer(BearerError),
}

impl From<BearerError> for NetworkError {
    fn from(err: BearerError) -> Self {
        NetworkError::Bearer(err)
    }
}

pub struct AdvertisingAndGattNetworkInterfaces<
    AB: AdvertisingBearer,
    GB: GattBearer<MTU>,
    const MTU: usize,
> {
    advertising_interface: AdvertisingBearerNetworkInterface<AB>,
    gatt_interface: GattBearerNetworkInterface<GB, MTU>,
}

impl<AB: AdvertisingBearer, GB: GattBearer<MTU>, const MTU: usize>
    AdvertisingAndGattNetworkInterfaces<AB, GB, MTU>
{
    pub fn new(advertising_bearer: AB, gatt_bearer: GB) -> Self {
        Self {
            advertising_interface: AdvertisingBearerNetworkInterface::new(advertising_bearer),
            gatt_interface: GattBearerNetworkInterface::new(gatt_bearer),
        }
    }
}

impl<AB: AdvertisingBearer, GB: GattBearer<MTU>, const MTU: usize> NetworkInterfaces
    for AdvertisingAndGattNetworkInterfaces<AB, GB, MTU>
{
    type RunFuture<'m> = impl Future<Output=Result<(), NetworkError>> + 'm
    where
    Self: 'm;

    fn run(&self) -> Self::RunFuture<'_> {
        self.gatt_interface.run()
    }

    type ReceiveFuture<'m> = impl Future<Output=Result<PDU, NetworkError>> + 'm
    where
    Self: 'm;

    fn receive<'m>(
        &'m self,
        state: &'m DeviceState,
        watchdog: &'m Watchdog,
    ) -> Self::ReceiveFuture<'m> {
        async move {
            let adv_fut = self.advertising_interface.receive(state, watchdog);
            let gatt_fut = self.gatt_interface.receive();
            let result = select(adv_fut, gatt_fut).await;

            match result {
                Either::First(result) | Either::Second(result) => Ok(result?),
            }
        }
    }

    type TransmitFuture<'m> = impl Future<Output=Result<(), NetworkError>> + 'm
    where
    Self: 'm;

    fn transmit<'m>(&'m self, pdu: &'m PDU, is_retransmit: bool) -> Self::TransmitFuture<'m> {
        async move {
            let gatt_fut = async {
                if !is_retransmit {
                    self.gatt_interface.transmit(pdu).await?;
                }

                Result::<(), NetworkError>::Ok(())
            };
            let adv_fut = self.advertising_interface.transmit(pdu);

            let _result = join(gatt_fut, adv_fut).await;
            Ok(())
        }
    }

    type BeaconFuture<'m> = impl Future<Output=Result<(), NetworkError>> + 'm
    where
    Self: 'm;

    fn beacon(&self, beacon: Beacon) -> Self::BeaconFuture<'_> {
        async move {
            self.advertising_interface.beacon(beacon).await?;
            self.gatt_interface.beacon(beacon).await?;
            Ok(())
        }
    }

    type CloseLinkFuture<'m> = impl Future<Output=Result<(), NetworkError>> + 'm
    where
    Self: 'm;

    fn close_link(&self, reason: Reason) -> Self::CloseLinkFuture<'_> {
        async move {
            self.advertising_interface.close_link(reason).await?;
            Ok(())
        }
    }

    fn reset(&self) {
        self.advertising_interface.reset();
        self.gatt_interface.reset();
    }
}

pub struct AdvertisingOnlyNetworkInterfaces<B: AdvertisingBearer> {
    interface: AdvertisingBearerNetworkInterface<B>,
}

impl<B: AdvertisingBearer> AdvertisingOnlyNetworkInterfaces<B> {
    pub fn new(bearer: B) -> Self {
        Self {
            interface: AdvertisingBearerNetworkInterface::new(bearer),
        }
    }
}

struct NeverEnding;
impl Future for NeverEnding {
    type Output = Result<(), NetworkError>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

impl<B: AdvertisingBearer> NetworkInterfaces for AdvertisingOnlyNetworkInterfaces<B> {
    type RunFuture<'m> = impl Future<Output=Result<(), NetworkError>> + 'm
    where
    Self: 'm;

    fn run(&self) -> Self::RunFuture<'_> {
        NeverEnding
    }

    type ReceiveFuture<'m> = impl Future<Output=Result<PDU, NetworkError>> + 'm
    where
    Self: 'm;

    fn receive<'m>(
        &'m self,
        state: &'m DeviceState,
        watchdog: &'m Watchdog,
    ) -> Self::ReceiveFuture<'m> {
        async move { Ok(self.interface.receive(state, watchdog).await?) }
    }

    type TransmitFuture<'m> = impl Future<Output=Result<(), NetworkError>> + 'm
    where
    Self: 'm;

    fn transmit<'m>(&'m self, pdu: &'m PDU, _is_retransmit: bool) -> Self::TransmitFuture<'m> {
        async move { Ok(self.interface.transmit(pdu).await?) }
    }

    type BeaconFuture<'m> = impl Future<Output=Result<(), NetworkError>> + 'm
    where
    Self: 'm;

    fn beacon(&self, beacon: Beacon) -> Self::BeaconFuture<'_> {
        async move { Ok(self.interface.beacon(beacon).await?) }
    }

    type CloseLinkFuture<'m> = impl Future<Output=Result<(), NetworkError>> + 'm
    where
    Self: 'm;

    fn close_link(&self, reason: Reason) -> Self::CloseLinkFuture<'_> {
        async move {
            self.interface.close_link(reason).await?;
            Ok(())
        }
    }

    fn reset(&self) {
        self.interface.reset();
    }
}
