#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(const_trait_impl)]
#![allow(incomplete_features)]

mod sensors;

use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Output, Level};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0, I2C0};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio };
use embassy_rp::i2c::InterruptHandler as I2CInterruptHandler;
use embassy_time::Timer;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};
//use crate::sensors::tlv493d::TLV493D;

bind_interrupts!(
    struct Irqs {
        PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
        I2C0_IRQ => I2CInterruptHandler<I2C0>;
    }
);


#[embassy_executor::task]
async fn wifi_task(runner: cyw43::Runner<'static, Output<'static, PIN_23>, PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");
    static STATE: StaticCell<cyw43::State> = StaticCell::new();

    //Inits clock (default at 12MHz), DMA, GPIOs, and Timers
    let rp2040 = embassy_rp::init(Default::default());
    let fw = include_bytes!("../Firmware/43439A0.bin");
    let clm = include_bytes!("../Firmware/43439A0_clm.bin");

    // Let us set up the CYW43 Wi-Fi chip
    // the pico uses GPIO 23, 24, 25, and 29 for Wi-Fi
    let pwr = Output::new(rp2040.PIN_23, Level::Low); // This is WL_ON for the pico W and maps to PIN 35 for the RP 2040
    let cs = Output::new(rp2040.PIN_25, Level::High); // This is WL_CS for the pico W and maps to PIN 37 for the RP 2040
    // The RP2040 has 2 PIO cores, each with 4 state machines that can be used to program for extra hardware.
    // These PIO cores operate separately from the CPU and communicate via FIFO buffers and Interrupt Requests
    let mut pio = Pio::new(rp2040.PIO0, Irqs); // Initialize our PIO core.
    // This sets up an SPI interface via PIO. Which leaves the RP2040's 2 SPI interfaces available.
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, rp2040.PIN_24, rp2040.PIN_29, rp2040.DMA_CH0);

    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;
    control.gpio_set(0, true).await;

    // let sda = rp2040.PIN_16;
    // let scl = rp2040.PIN_17;
    //
    // let i2c = i2c::I2c::new_async(rp2040.I2C0, scl, sda, Irqs, Config::default());
    // const TLV493D_ADDR: u16 = 0xBD;
    //
    // let mut sensor = TLV493D::new(i2c, TLV493D_ADDR);
    // sensor.init().await;
    //
    // loop {
    //     let vals = sensor.get_sensor_reading().await;
    //     println!("{:?}", vals);
    //     Timer::after_millis(50).await;
    // }

    let mut on = false;
    loop {
        if on {
            control.gpio_set(0, false).await;
            on = false;
        } else {
            control.gpio_set(0, true).await;
            on = true;
        }

        info!("Hello!");
        Timer::after_millis(50).await;
    }
}
