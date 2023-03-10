//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use bsp::{entry, hal::pio::PinState};
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;

// Embed the `Hz` function/trait:
use fugit::RateExtU32;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio, pac,
    sio::Sio,
    spi,
    watchdog::Watchdog,
};

use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text, TextStyle},
};

use display_interface_spi::SPIInterface;
use ili9341::{Ili9341, Orientation};

type Display = Ili9341<
    SPIInterface<
        spi::Spi<spi::Enabled, pac::SPI1, 8>,
        gpio::Pin<gpio::bank0::Gpio15, gpio::Output<gpio::PushPull>>,
        gpio::Pin<gpio::bank0::Gpio13, gpio::Output<gpio::PushPull>>,
    >,
    gpio::Pin<gpio::bank0::Gpio14, gpio::Output<gpio::PushPull>>,
>;

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.led.into_push_pull_output();
    let mut display: Display = {
        let cs = pins.gpio13.into_push_pull_output();
        let dc = pins.gpio15.into_push_pull_output();
        let reset = pins.gpio14.into_push_pull_output();
        let _sclk = pins.gpio10.into_mode::<gpio::FunctionSpi>();
        let _mosi = pins.gpio11.into_mode::<gpio::FunctionSpi>();
        let _miso = pins.gpio12.into_mode::<gpio::FunctionSpi>();

        let spi = spi::Spi::<_, _, 8>::new(pac.SPI1);
        let spi = spi.init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            16_000_000u32.Hz(),
            &embedded_hal::spi::MODE_0,
        );
        Ili9341::new(
            SPIInterface::new(spi, dc, cs),
            reset,
            &mut delay,
            Orientation::LandscapeFlipped,
            ili9341::DisplaySize240x320,
        )
        .unwrap()
    };
    let in1_pin = pins.gpio16.into_pull_up_input();

    let character_style = MonoTextStyle::new(&profont::PROFONT_24_POINT, Rgb565::WHITE);
    let text_style = TextStyle::with_baseline(Baseline::Top);
    let black_fill = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLACK)
        .build();
    let test_text = "Pixel Blinky";

    Rectangle::new(Point::new(0, 0), Size::new(320, 240))
        .into_styled(black_fill)
        .draw(&mut display)
        .unwrap();

    Text::with_text_style(test_text, Point::new(60, 0), character_style, text_style)
        .draw(&mut display)
        .unwrap();

    let mut blink = false;

    loop {
        if blink {
            led_pin.set_high().unwrap();
        } else {
            led_pin.set_low().unwrap();
        }
        render_indicator(&mut display, Point::new(120, 120), blink);
        render_indicator(
            &mut display,
            Point::new(180, 120),
            in1_pin.is_high().unwrap(),
        );

        blink = !blink;
        delay.delay_ms(500);
    }
}

fn render_indicator(display: &mut Display, centre: Point, state: bool) -> () {
    let led_size: u32 = 30;
    let led_at = Point::new(
        centre.x - (led_size as i32) / 2,
        centre.y - (led_size as i32) / 2,
    );

    let grey_fill = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::CSS_DARK_GRAY)
        .build();
    let green_fill = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::GREEN)
        .build();

    if state {
        Circle::new(led_at, led_size)
            .into_styled(green_fill)
            .draw(display)
            .unwrap();
    } else {
        Circle::new(led_at, led_size)
            .into_styled(grey_fill)
            .draw(display)
            .unwrap();
    }
}

// End of file
