#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use embassy::executor::Spawner;
use embassy_nrf::Peripherals;

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

#[embassy::main(config = "config()")]
async fn main(_spawner: Spawner, p: Peripherals) {
    let mut driver = Driver::new(
        "drogue",
        unsafe { &__storage as *const u8 as u32 },
        100,
    );

    let mut device = Device::new(
        p.P0_13.degrade(),
        p.P0_11.degrade(),
    );
    driver.run(&mut device).await.unwrap();
}

// Application must run at a lower priority than softdevice
fn config() -> Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    config
}
