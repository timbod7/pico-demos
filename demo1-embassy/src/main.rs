#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::cell::RefCell;
use defmt::*;
use display::Display;
use embassy_executor::Spawner;
use embassy_rp::{gpio, peripherals};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use gpio::{Input, Level, Output, Pull};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    prelude::*,
    pixelcolor::Rgb565,
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle},
    text::Text,
};

mod display;

type LedOutput = Output<'static, peripherals::PIN_25>;
type ButtonInput = Input<'static, peripherals::PIN_16>;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Program start");

    let p = embassy_rp::init(Default::default());
    let led: LedOutput = Output::new(p.PIN_25, Level::Low);
    let button: ButtonInput = Input::new(p.PIN_16, Pull::Up);

    let display = display::init(
        p.PIN_12, p.PIN_11, p.PIN_10, p.PIN_13, p.PIN_14, p.PIN_15, p.SPI1,
    );

    unwrap!(spawner.spawn(blinker(led, Duration::from_millis(500))));
    unwrap!(spawner.spawn(button_monitor(button)));
    unwrap!(spawner.spawn(display_refresh(display)));
}

/// Blink the physical LED, and a matching indicator on the LCD display
#[embassy_executor::task]
async fn blinker(mut led: LedOutput, interval: Duration) {
    let mut blink = false;
    loop {
        led.set_level(if blink { Level::Low } else { Level::High });
        display_state_update(|mut s| s.indicator1 = blink);
        blink = !blink;
        Timer::after(interval).await;
    }
}

/// Monitor the button, and show an indicator on the LCD display
#[embassy_executor::task]
async fn button_monitor(mut button: ButtonInput) {
    loop {
        button.wait_for_any_edge().await;
        let level = button.get_level();
        display_state_update(|mut s| s.indicator2 = level == Level::High);
    }
}

#[derive(Clone)]
struct DisplayState {
    indicator1: bool,
    indicator2: bool,
}

static DISPLAY_STATE: Mutex<ThreadModeRawMutex, RefCell<DisplayState>> =
    Mutex::new(RefCell::new(DisplayState {
        indicator1: false,
        indicator2: false,
    }));
static DISPLAY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

// Keep the display up to date
#[embassy_executor::task]
async fn display_refresh(mut display: Display) {
    render_background(&mut display);
    loop {
        DISPLAY_SIGNAL.wait().await;
        let state = DISPLAY_STATE.lock(|s| s.borrow().clone());
        render_indicator(&mut display, Point::new(120, 120), state.indicator1);
        render_indicator(&mut display, Point::new(180, 120), state.indicator2);
    }
}

fn display_state_update<F>(mut sfn: F)
where
    F: FnMut(&mut DisplayState) -> (),
{
    DISPLAY_STATE.lock(|s| sfn(&mut s.borrow_mut()));
    DISPLAY_SIGNAL.signal(());
}

fn render_background(display: &mut Display) {
    let test_text = "Pixel Blinky";

    Rectangle::new(Point::new(0, 0), Size::new(320, 240))
        .into_styled(display.styles.black_fill)
        .draw(&mut display.interface)
        .unwrap();

    Text::with_text_style(
        test_text,
        Point::new(60, 0),
        display.styles.char,
        display.styles.text,
    )
    .draw(&mut display.interface)
    .unwrap();
}

/// Draw an "LED" on the LCD display
fn render_indicator(display: &mut Display, centre: Point, state: bool) -> () {
    let led_size: u32 = 30;
    let led_at = Point::new(
        centre.x - (led_size as i32) / 2,
        centre.y - (led_size as i32) / 2,
    );

    if state {
        Circle::new(led_at, led_size)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb565::GREEN)
                    .build(),
            )
            .draw(&mut display.interface)
            .unwrap();
    } else {
        Circle::new(led_at, led_size)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb565::CSS_DARK_GRAY)
                    .build(),
            )
            .draw(&mut display.interface)
            .unwrap();
    }
}
