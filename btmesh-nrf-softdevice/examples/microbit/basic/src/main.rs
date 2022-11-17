#![no_std]
#![no_main]
#![macro_use]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;

use embassy_nrf::config::Config;
use embassy_nrf::interrupt::Priority;

use btmesh_nrf_softdevice::*;

use defmt_rtt as _;
use embassy_nrf::gpio::Pin;
use panic_probe as _;

mod device;

use device::Device;

extern "C" {
    static __storage: u8;
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(config());
    let mut driver = Driver::new(
        "drogue",
        unsafe { &__storage as *const u8 as u32 },
        None,
        100,
        Default::default(),
    );

    let mut device = Device::new(p.P0_13.degrade(), p.P0_11.degrade());
    driver.run(&mut device).await.unwrap();
}

// Application must run at a lower priority than softdevice
fn config() -> Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    config
}
