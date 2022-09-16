#![no_std]
#![no_main]

use core::cell::RefCell;

use hal::{
    gpio::{Gpiob, Gpioc, Pin, U},
    interrupt,
};

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use core::fmt::Write;
use cortex_m::{
    asm,
    interrupt::{CriticalSection, Mutex},
    peripheral::NVIC,
};
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::hio;
use cortex_m_semihosting::hprintln;

use stm32f3xx_hal::{
    self as hal,
    gpio::{Edge, Input, Output, PushPull},
    pac,
    prelude::*,
};

type LedPin = Pin<Gpiob, U<13>, Output<PushPull>>;
static LED: Mutex<RefCell<Option<LedPin>>> = Mutex::new(RefCell::new(None));
type ButtonPin = Pin<Gpioc, U<13>, Input>;
static BUTTON: Mutex<RefCell<Option<ButtonPin>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut rcc = dp.RCC.constrain();
    let mut syscfg = dp.SYSCFG.constrain(&mut rcc.apb2);
    let mut exti = dp.EXTI;

    // grab the led
    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    let led: Pin<Gpiob, U<13>, Output<PushPull>> = gpiob
        .pb13
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);

    // store it so we can share with interrupt
    cortex_m::interrupt::free(|cs| *LED.borrow(cs).borrow_mut() = Some(led));

    // grab the button
    let mut gpioc = dp.GPIOC.split(&mut rcc.ahb);
    let mut button = gpioc
        .pc13
        .into_pull_down_input(&mut gpioc.moder, &mut gpioc.pupdr);
    syscfg.select_exti_interrupt_source(&button);
    button.trigger_on_edge(&mut exti, Edge::Rising);
    button.enable_interrupt(&mut exti);
    let button_interrupt_num = button.interrupt();

    // store it off
    cortex_m::interrupt::free(|cs| *BUTTON.borrow(cs).borrow_mut() = Some(button));
    unsafe { NVIC::unmask(button_interrupt_num) };

    let delay = 1_000_000;

    loop {
        hprintln!("loop!").unwrap();
        cortex_m::interrupt::free(toggle_led);
        asm::delay(delay);
    }
}

fn toggle_led(cs: &CriticalSection) {
    LED.borrow(cs)
        .borrow_mut()
        .as_mut()
        .unwrap()
        .toggle()
        .unwrap()
}

fn toggle_led_from_interrupt(cs: &CriticalSection) {
    toggle_led(cs);
}

#[interrupt]
fn EXTI15_10() {
    cortex_m::interrupt::free(|cs| {
        toggle_led_from_interrupt(cs);

        // Clear the interrupt pending bit so we don't infinitely call this routine
        BUTTON
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .clear_interrupt();
    })
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    if let Ok(mut hstdout) = hio::hstdout() {
        writeln!(hstdout, "we hit an exception {:#?}", irqn).ok();
    }

    loop {}
}
