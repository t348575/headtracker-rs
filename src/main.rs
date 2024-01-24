#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::panic::PanicInfo;

use cortex_m_rt::{exception, ExceptionFrame};
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::interrupt;
use embassy_stm32::time::Hertz;
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::i2c;
use embassy_time::{Duration, Timer, Instant};
use embedded_hal_async::i2c::I2c;
use rtt_target::{rprintln, rtt_init_print};

use crate::gy87::Gy87;
use crate::wifi::Wifi;

mod constants;
mod gy87;
mod util;
mod wifi;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    rtt_init_print!();
    let p = embassy_stm32::init(Default::default());

    rprintln!("chip up!");

    let config = Config::default();
    let irq = interrupt::take!(USART1);
    let usart = Uart::new(p.USART1, p.PA10, p.PA9, irq, p.DMA2_CH7, p.DMA2_CH5, config);

    Timer::after(Duration::from_millis(1000)).await;
    let mut wifi = Wifi::new(usart);
    let wifi_connected = wifi.setup().await.unwrap();
    if !wifi_connected {
        wifi.connect().await.unwrap();
    }
    Timer::after(Duration::from_millis(1000)).await;
    wifi.start_udp().await.unwrap();

    rprintln!("wifi up!");

    let irq = interrupt::take!(I2C1_EV);
    let i2c = i2c::I2c::new(
        p.I2C1,
        p.PB6,
        p.PB7,
        irq,
        p.DMA1_CH0,
        p.DMA1_CH6,
        Hertz(400_000),
        Default::default(),
    );

    let mut gy87 = Gy87::new(i2c);
    gy87.start().unwrap();

    rprintln!("gy87 up!");

    let mut led = Output::new(p.PC13, Level::Low, Speed::Low);
    led.set_low();

    let mut prev = Instant::now();
    let mut i = 0;
    loop {
        if let Ok(data) = gy87.update(&prev) {
            prev = Instant::now();
            let data = data.serialize();
            match wifi.send_pos_data(&data).await {
                Err(err) => rprintln!("{:?}", err),
                _ => {}
            }
        }
    }
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rprintln!("{}", info);
    loop {}
}
