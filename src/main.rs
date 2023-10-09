#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use alloc::string::String;
use embassy_executor::Executor;
use embassy_net::{Config, Stack, StackResources};
use embassy_time::{Duration, Timer};
use embedded_storage::{ReadStorage, Storage};
use esp32c3_hal::{
    clock::ClockControl,
    gpio::{Bank0GpioRegisterAccess, SingleCoreInteruptStatusRegisterAccessBank0, Unknown},
    prelude::*,
    pulse_control::ClockSource,
    soc::{gpio::Gpio8Signals, peripherals::Peripherals},
    timer::TimerGroup,
    PulseControl, Rtc, IO,
};
use esp_backtrace as _;
use esp_hal_common::{
    gpio::{GpioPin, InputOutputPinType},
    pulse_control::Channel0,
};
use esp_hal_smartled::{smartLedAdapter, SmartLedsAdapter};
use esp_storage::FlashStorage;
use esp_wifi::wifi::{WifiController, WifiMode};
use smart_leds::{
    brightness, gamma,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite,
};
use static_cell::StaticCell;

mod ws281x;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 2 * 1024;

    extern "C" {
        static mut _heap_start: u32;
    }
    unsafe {
        let heap_start = &_heap_start as *const _ as usize;
        ALLOCATOR.init(heap_start as *mut u8, HEAP_SIZE);
    }
}

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[embassy_executor::task]
async fn run_leds(
    pulse: Channel0,
    pin: GpioPin<
        Unknown,
        Bank0GpioRegisterAccess,
        SingleCoreInteruptStatusRegisterAccessBank0,
        InputOutputPinType,
        Gpio8Signals,
        8,
    >,
) {
    let mut led = <smartLedAdapter!(1)>::new(pulse, pin);
    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };

    let mut data;
    loop {
        for hue in 0..=255 {
            color.hue = hue;
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED
            data = [hsv2rgb(color)];
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 10 out of 255 so
            // that the output it's not too bright.
            led.write(brightness(gamma(data.iter().cloned()), 10))
                .unwrap();
            Timer::after(Duration::from_millis(20)).await;
        }
    }
}

// Found some areas that are "free"
// Or overwrite our own program memory.
const WIFI_INFO_OFFSET: u32 = 0xc_000;

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    let mut flash = FlashStorage::new();
    esp_println::println!("Flash storage capacity: {}", flash.capacity());
    let mut data = [0; 128];
    let result = flash.read(WIFI_INFO_OFFSET, &mut data);
    esp_println::println!("Result from reading memory: {result:?}");
    let _ = result.unwrap();
    esp_println::println!("Flash data at offset: {:?}", data);
    esp_println::println!("Flash data at offset: {}", String::from_utf8_lossy(&data));
    loop {
        esp_println::println!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}

#[entry]
fn main() -> ! {
    init_heap();
    esp_println::println!("Starting!");
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    #[cfg(feature = "embassy-time-systick")]
    esp_hal_common::embassy::init(
        &clocks,
        esp32c3_hal::systimer::SystemTimer::new(peripherals.SYSTIMER),
    );

    #[cfg(feature = "embassy-time-timg0")]
    esp_hal_common::embassy::init(&clocks, timer_group0.timer0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let pulse = PulseControl::new(
        peripherals.RMT,
        &mut system.peripheral_clock_control,
        ClockSource::APB,
        0,
        0,
        0,
    )
    .unwrap();

    let (wifi, _ble) = peripherals.RADIO.split();
    let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(wifi, WifiMode::Sta);

    let config = Config::Dhcp(Default::default());

    let stack = &*singleton!(Stack::new(
        wifi_interface,
        config,
        singleton!(StackResources::<3>::new()),
        1234
    ));

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(run_leds(pulse.channel0, io.pins.gpio8)).ok();
        spawner.spawn(connection(controller)).ok();
    });
}
