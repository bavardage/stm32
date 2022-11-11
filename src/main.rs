#![no_std]
#![no_main]

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use core::{cell::RefCell, fmt::Write, ops::DerefMut};
use cortex_m::{
    interrupt::{free, Mutex},
    peripheral::NVIC,
};
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::hio;
// use cortex_m_semihosting::hprintln;

use stm32h7xx_hal::{
    gpio::gpioc::PC13,
    gpio::{Edge, ExtiPin, Input},
    interrupt, pac,
    prelude::*,
};

static BUTTON: Mutex<RefCell<Option<PC13<Input>>>> = Mutex::new(RefCell::new(None));
static MODE: Mutex<RefCell<u8>> = Mutex::new(RefCell::new(0_u8));

#[entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    let rcc = dp.RCC.constrain();
    let ccdr = rcc.sys_ck(100.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    let mut syscfg = dp.SYSCFG;
    let mut exti = dp.EXTI;

    // grab the leds
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);
    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);

    let mut led1 = gpiob.pb0.into_push_pull_output();
    let mut led2 = gpioe.pe1.into_push_pull_output();
    let mut led3 = gpiob.pb14.into_push_pull_output();

    let mut button1 = gpioc.pc13.into_pull_down_input();
    button1.make_interrupt_source(&mut syscfg);
    button1.trigger_on_edge(&mut exti, Edge::Rising);
    button1.enable_interrupt(&mut exti);

    free(|cs| {
        BUTTON.borrow(cs).replace(Some(button1));
    });

    unsafe {
        cp.NVIC.set_priority(interrupt::EXTI15_10, 1);
        NVIC::unmask::<interrupt>(interrupt::EXTI15_10);
    }

    let mut delay = cp.SYST.delay(ccdr.clocks);

    let mut state: u8 = 0;

    loop {
        // hprintln!("loop! state: {:?}", state).unwrap();
        delay.delay_ms(125_u16);

        let mode = free(|cs| {
            *MODE.borrow(cs).borrow()
        });

        if mode % 2 == 0 {
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
        } else {
            led1.toggle();
            led2.toggle();
            led3.toggle();
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

#[interrupt]
fn EXTI15_10() {
    // hprintln!("EXTI3 fired!").unwrap();

    free(|cs| {
        if let Some(ref mut btn) = BUTTON.borrow(cs).borrow_mut().deref_mut() {
            btn.clear_interrupt_pending_bit();
        }

        let ref mut mode = MODE.borrow(cs).borrow_mut();
        let current_mode = **mode;

        **mode = current_mode.wrapping_add(1);
    })
}
