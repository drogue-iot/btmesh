use crate::advertising::SoftdeviceAdvertisingBearer;
use crate::gatt::SoftdeviceGattBearer;
use crate::rng::SoftdeviceRng;
use btmesh_driver::stack::interface::{
    AdvertisingAndGattNetworkInterfaces, AdvertisingOnlyNetworkInterfaces, NetworkInterfaces,
};
use btmesh_driver::storage::flash::FlashBackingStore;
use btmesh_driver::{BluetoothMeshDriver, DriverError, Driver as BaseDriver};
use core::future::{Future, join};
use core::mem;
use btmesh_device::BluetoothMeshDevice;
use nrf_softdevice::{raw, Flash, Softdevice};

pub struct NrfSoftdeviceDriverBuilder {
    sd: &'static Softdevice,
}

impl NrfSoftdeviceDriverBuilder {
    pub fn new(name: &'static str) -> Self {
        Self {
            sd: Self::new_sd(name),
        }
    }

    fn new_sd(device_name: &'static str) -> &'static Softdevice {
        let config = nrf_softdevice::Config {
            clock: Some(raw::nrf_clock_lf_cfg_t {
                source: raw::NRF_CLOCK_LF_SRC_RC as u8,
                rc_ctiv: 4,
                rc_temp_ctiv: 2,
                accuracy: 7,
            }),
            conn_gap: Some(raw::ble_gap_conn_cfg_t {
                conn_count: 1,
                event_length: 24,
            }),
            conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 517 }),
            gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
                attr_tab_size: 32768,
            }),
            gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
                adv_set_count: 1,
                periph_role_count: 2,
                central_role_count: 2,
                central_sec_count: 2,
                _bitfield_1: Default::default(),
            }),
            gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
                p_value: device_name.as_ptr() as *const u8 as _,
                current_len: device_name.len() as u16,
                max_len: device_name.len() as u16,
                write_perm: unsafe { mem::zeroed() },
                _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                    raw::BLE_GATTS_VLOC_STACK as u8,
                ),
            }),

            ..Default::default()
        };
        Softdevice::enable(&config)
    }

    fn sd(&self) -> &'static Softdevice {
        self.sd
    }

    fn advertising_bearer(&self) -> SoftdeviceAdvertisingBearer {
        SoftdeviceAdvertisingBearer::new(self.sd)
    }

    fn gatt_bearer(&self) -> SoftdeviceGattBearer {
        SoftdeviceGattBearer::new(self.sd)
    }

    fn rng(&self) -> SoftdeviceRng {
        SoftdeviceRng::new(self.sd)
    }

    fn flash(&self) -> Flash {
        Flash::take(self.sd)
    }
}

pub struct NrfSoftdeviceDriver<N: NetworkInterfaces> {
    sd: &'static Softdevice,
    driver: BaseDriver<N, SoftdeviceRng, FlashBackingStore<Flash>>,
}

impl<N: NetworkInterfaces> NrfSoftdeviceDriver<N> {
    pub fn new(
        sd: &'static Softdevice,
        network: N,
        rng: SoftdeviceRng,
        backing_store: FlashBackingStore<Flash>,
    ) -> Self {
        Self {
            sd,
            driver: BaseDriver::new(network, rng, backing_store),
        }
    }

    #[allow(unreachable_code)]
    pub async fn run<'r, D: BluetoothMeshDevice>(&'r mut self, device: &'r mut D) -> Result<(), DriverError> {
        // todo: turn it into a select?
        join!( self.sd.run(), self.driver.run(device)).await.1
    }
}

pub struct NrfSoftdeviceAdvertisingOnlyDriver(
    NrfSoftdeviceDriver<AdvertisingOnlyNetworkInterfaces<SoftdeviceAdvertisingBearer>>,
);

impl NrfSoftdeviceAdvertisingOnlyDriver {
    pub fn new(
        name: &'static str,
        base_address: u32,
        sequence_threshold: u32,
    ) -> Self {
        let builder = NrfSoftdeviceDriverBuilder::new(name);
        let rng = builder.rng();
        let backing_store =
            FlashBackingStore::new(builder.flash(), base_address, sequence_threshold);
        let adv_bearer = builder.advertising_bearer();

        let network = AdvertisingOnlyNetworkInterfaces::new(adv_bearer);

        Self(NrfSoftdeviceDriver::new(
            builder.sd,
            network,
            rng,
            backing_store,
        ))
    }

    pub async fn run<'r, D: BluetoothMeshDevice>(&'r mut self, device: &'r mut D) -> Result<(), DriverError> {
        self.0.run(device).await
    }
}

impl BluetoothMeshDriver for NrfSoftdeviceAdvertisingOnlyDriver {
    type RunFuture<'f, D> = impl Future<Output=Result<(), DriverError>> + 'f
    where
    Self: 'f, D: BluetoothMeshDevice + 'f;

    fn run<'r, D: BluetoothMeshDevice>(&'r mut self, device: &'r mut D) -> Self::RunFuture<'_, D> {
        self.0.run(device)
    }
}

pub struct NrfSoftdeviceAdvertisingAndGattDriver(
    NrfSoftdeviceDriver<
        AdvertisingAndGattNetworkInterfaces<SoftdeviceAdvertisingBearer, SoftdeviceGattBearer, 66>,
    >,
);

impl NrfSoftdeviceAdvertisingAndGattDriver {
    pub fn new(
        name: &'static str,
        base_address: u32,
        sequence_threshold: u32,
    ) -> Self {
        let builder = NrfSoftdeviceDriverBuilder::new(name);
        let rng = builder.rng();
        let backing_store =
            FlashBackingStore::new(builder.flash(), base_address, sequence_threshold);
        let adv_bearer = builder.advertising_bearer();
        let gatt_bearer = builder.gatt_bearer();

        let network = AdvertisingAndGattNetworkInterfaces::new(adv_bearer, gatt_bearer);

        Self(NrfSoftdeviceDriver::new(
            builder.sd,
            network,
            rng,
            backing_store,
        ))
    }
}

impl BluetoothMeshDriver for NrfSoftdeviceAdvertisingAndGattDriver {
    type RunFuture<'f, D> = impl Future<Output=Result<(), DriverError>> + 'f
    where
    Self: 'f, D: BluetoothMeshDevice + 'f;

    fn run<'r, D: BluetoothMeshDevice>(&'r mut self, device: &'r mut D) -> Self::RunFuture<'_, D> {
        self.0.run(device)
    }
}


