#![no_std]
#![no_main]

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use core::fmt::Write;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::hio;

use stm32h7xx_hal::{
    pac,
    prelude::*,
};

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    let rcc = dp.RCC.constrain();
    let ccdr = rcc.sys_ck(100.MHz()).freeze(pwrcfg, &dp.SYSCFG);


    // grab the leds
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);

    let mut led1 = gpiob
        .pb0
        .into_push_pull_output();
    let mut led2 = gpioe.pe1.into_push_pull_output();
    let mut led3 = gpiob.pb14.into_push_pull_output();

    let mut delay = cp.SYST.delay(ccdr.clocks);

    let mut state: u8 = 0;

    loop {
        // hprintln!("loop! state: {:?}", state).unwrap();
        delay.delay_ms(125_u16);

        if state & 0x1 != 0 {
            led1.set_high();
        } else {
            led1.set_low();
        }

        if state & 0x2 != 0 {
            led2.set_high();
        } else {
            led2.set_low();
        }

        if state & 0x4 != 0 {
            led3.set_high();
        } else {
            led3.set_low();
        }

        state = (state + 1) % 8;
    }
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    if let Ok(mut hstdout) = hio::hstdout() {
        writeln!(hstdout, "we hit an exception {:#?}", irqn).ok();
    }

    loop {}
}
