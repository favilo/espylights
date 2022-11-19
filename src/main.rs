#![no_std]
#![no_main]

extern crate alloc;
use esp32c3_hal::{
    clock::ClockControl,
    pac::Peripherals,
    prelude::*,
    pulse_control::{ClockSource, OutputChannel, PulseCode},
    timer::TimerGroup,
    Delay, PulseControl, Rtc, IO,
};
use esp_backtrace as _;
use palette::Srgb;

mod ws281x;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[allow(dead_code)]
fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;

    extern "C" {
        static mut _heap_start: u32;
    }

    unsafe {
        let heap_start = &_heap_start as *const _ as usize;
        ALLOCATOR.init(heap_start as *mut u8, HEAP_SIZE);
    }
}
#[riscv_rt::entry]
fn main() -> ! {
    init_heap();
    let peripherals = Peripherals::take().unwrap();
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // Set GPIO7 as an output, and set its state high initially.
    // Configure RMT peripheral globally
    let pulse = PulseControl::new(
        peripherals.RMT,
        &mut system.peripheral_clock_control,
        ClockSource::APB,
        0,
        0,
        0,
    )
    .unwrap();

    let mut rmt_channel0 = pulse.channel0;

    // Set up channel
    rmt_channel0
        .set_idle_output_level(false)
        .set_carrier_modulation(false)
        .set_channel_divider(1)
        .set_idle_output(true);

    let rmt_channel0 = rmt_channel0.assign_pin(io.pins.gpio8);
    let mut ws2811 = ws281x::Ws281X::new(rmt_channel0);

    let mut seq = [PulseCode {
        level1: true,
        length1: 0u32.nanos(),
        level2: false,
        length2: 0u32.nanos(),
    }; 128];

    // -1 to make sure that the last element is a transmission end marker (i.e.
    // lenght 0)
    for i in 0..(seq.len() - 1) {
        seq[i] = PulseCode {
            level1: true,
            length1: (10u32 * (i as u32 + 1u32)).nanos(),
            level2: false,
            length2: 60u32.nanos(),
        };
    }

    // Initialize the Delay peripheral, and use it to toggle the LED state in a
    // loop.
    let mut delay = Delay::new(&clocks);

    loop {
        delay.delay_ms(500u32);
        // Send sequence
        ws2811.send_colors(&[Srgb::<u8>::new(255, 0, 0)]).unwrap();
        // rmt_channel0
        //     .send_pulse_sequence(RepeatMode::SingleShot, &seq)
        //     .unwrap();
    }
}
