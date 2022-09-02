use atomic_polyfill::AtomicBool;
use btmesh_bearer::{BearerError, GattBearer};
use core::cell::RefCell;
use core::future::Future;
use core::sync::atomic::Ordering;
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::{channel::Channel, signal::Signal};
use heapless::Vec;
use nrf_softdevice::ble::peripheral::AdvertiseError;
use nrf_softdevice::ble::{gatt_server, peripheral, Connection};
use nrf_softdevice::Softdevice;

static RESET_SIGNAL: Signal<()> = Signal::new();

pub enum ConnectionChannel {
    Provisioning,
    Proxy,
}

pub struct SoftdeviceGattBearer {
    sd: &'static Softdevice,
    connection: Signal<Connection>,
    current_connection: RefCell<Option<Connection>>,
    connection_channel: RefCell<Option<ConnectionChannel>>,
    server: MeshGattServer,
    connected: AtomicBool,
    outbound: Channel<ThreadModeRawMutex, Vec<u8, 66>, 5>,
    inbound: Channel<ThreadModeRawMutex, Vec<u8, 66>, 5>,
}

impl SoftdeviceGattBearer {
    pub fn new(sd: &'static Softdevice, server: MeshGattServer) -> Self {
        Self {
            sd,
            server,
            connection: Signal::new(),
            connected: AtomicBool::new(false),
            current_connection: RefCell::new(None),
            connection_channel: RefCell::new(None),
            outbound: Channel::new(),
            inbound: Channel::new(),
        }
    }

    #[allow(clippy::await_holding_refcell_ref)]
    async fn run(&self) -> Result<(), BearerError> {
        loop {
            let connection = self.connection.wait().await;
            self.current_connection.borrow_mut().replace(connection);

            let server_fut = async move {
                gatt_server::run(
                    self.current_connection.borrow().as_ref().unwrap(),
                    &self.server,
                    |e| match e {
                        #[cfg(feature = "proxy")]
                        MeshGattServerEvent::Proxy(event) => match event {
                            ProxyServiceEvent::DataInWrite(data) => {
                                self.inbound.try_send(data).ok();
                            }
                            ProxyServiceEvent::DataOutCccdWrite { notifications } => {
                                if notifications {
                                    self.connection_channel
                                        .replace(Some(ConnectionChannel::Proxy));
                                } else {
                                    self.connection_channel.take();
                                }
                            }
                            _ => { /* ignorable */ }
                        },
                        MeshGattServerEvent::Provisioning(event) => match event {
                            ProvisioningServiceEvent::DataInWrite(data) => {
                                self.inbound.try_send(data).ok();
                            }
                            ProvisioningServiceEvent::DataOutCccdWrite { notifications } => {
                                if notifications {
                                    self.connection_channel
                                        .replace(Some(ConnectionChannel::Provisioning));
                                } else {
                                    self.connection_channel.take();
                                }
                            }
                            _ => { /* ignorable */ }
                        },
                    },
                )
                .await
                .ok()
            };

            let reset_fut = RESET_SIGNAL.wait();

            select(server_fut, reset_fut).await;

            self.connection_channel.borrow_mut().take();
            self.current_connection.borrow_mut().take();
            self.connected.store(false, Ordering::Relaxed);
        }
    }
}

pub const ATT_MTU: usize = 69;

impl GattBearer<66> for SoftdeviceGattBearer {
    fn reset(&self) {
        RESET_SIGNAL.signal(())
    }

    type RunFuture<'m> = impl Future<Output=Result<(), BearerError>> + 'm
    where
    Self: 'm;

    fn run(&self) -> Self::RunFuture<'_> {
        SoftdeviceGattBearer::run(self)
    }

    type ReceiveFuture<'m> = impl Future<Output=Result<Vec<u8, 66>, BearerError>> + 'm
    where
    Self: 'm;

    fn receive(&self) -> Self::ReceiveFuture<'_> {
        async move { Ok(self.inbound.recv().await) }
    }

    type TransmitFuture<'m> = impl Future<Output = Result<(), BearerError>> + 'm;

    fn transmit<'m>(&'m self, pdu: &'m Vec<u8, 66>) -> Self::TransmitFuture<'m> {
        async move {
            if let Some(connection) = &*self.current_connection.borrow() {
                match &*self.connection_channel.borrow() {
                    Some(ConnectionChannel::Provisioning) => {
                        self.server
                            .provisioning
                            .data_out_notify(connection, pdu.clone())
                            .map_err(|_| BearerError::TransmissionFailure)?;
                    }
                    #[cfg(feature = "proxy")]
                    Some(ConnectionChannel::Proxy) => {
                        self.server
                            .proxy
                            .data_out_notify(connection, pdu.clone())
                            .map_err(|_| BearerError::TransmissionFailure)?;
                    }
                    _ => {}
                }
            }

            Ok(())
        }
    }

    type AdvertiseFuture<'m> = impl Future<Output = Result<(), BearerError>> + 'm;

    fn advertise<'m>(&'m self, adv_data: &'m Vec<u8, 64>) -> Self::AdvertiseFuture<'m> {
        async move {
            let adv_data = adv_data.clone();
            if self.connected.load(Ordering::Relaxed) {
                return Ok(());
            }

            let scan_data: Vec<u8, 16> = Vec::new();

            let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
                adv_data: &adv_data,
                scan_data: &scan_data,
            };

            let result = peripheral::advertise_connectable(
                self.sd,
                adv,
                &peripheral::Config {
                    timeout: Some(5),
                    interval: 50,
                    ..Default::default()
                },
            )
            .await;

            match result {
                Ok(connection) => {
                    self.connected.store(true, Ordering::Relaxed);
                    self.connection.signal(connection);
                    return Ok(());
                }
                Err(err) => match err {
                    AdvertiseError::Timeout => {}
                    AdvertiseError::NoFreeConn => {}
                    AdvertiseError::Raw(_) => {}
                },
            }
            Ok(())
        }
    }
}

#[cfg(not(feature = "proxy"))]
#[nrf_softdevice::gatt_server]
pub struct MeshGattServer {
    provisioning: ProvisioningService,
}

#[cfg(feature = "proxy")]
#[nrf_softdevice::gatt_server]
pub struct MeshGattServer {
    provisioning: ProvisioningService,
    proxy: ProxyService,
}

#[nrf_softdevice::gatt_service(uuid = "1827")]
pub struct ProvisioningService {
    #[characteristic(uuid = "2adb", write_without_response)]
    pub data_in: Vec<u8, 66>,
    #[characteristic(uuid = "2adc", read, write, notify)]
    pub data_out: Vec<u8, 66>,
}

#[cfg(feature = "proxy")]
#[nrf_softdevice::gatt_service(uuid = "1828")]
pub struct ProxyService {
    #[characteristic(uuid = "2add", write_without_response)]
    pub data_in: Vec<u8, 66>,
    #[characteristic(uuid = "2ade", read, write, notify)]
    pub data_out: Vec<u8, 66>,
}
