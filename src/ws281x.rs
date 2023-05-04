use esp32c3_hal::{
    prelude::*,
    pulse_control::{ConfiguredChannel, PulseCode, RepeatMode, TransmissionError},
};
use esp_println::println;
use lazy_static::lazy_static;
use palette::{cast::Packed, rgb::{Rgba, channels::Argb}, Srgb};

lazy_static! {
    static ref BITS: [PulseCode; 2] = {
        [
            PulseCode {
                level1: true,
                length1: 350u32.nanos(),
                level2: false,
                length2: 900u32.nanos(),
            },
            PulseCode {
                level1: true,
                length1: 600u32.nanos(),
                level2: false,
                length2: 650u32.nanos(),
            },
        ]
    };
}

pub struct Ws281X<Channel> {
    channel: Channel,
    bits: [PulseCode; 2],
}

impl<Channel: ConfiguredChannel> Ws281X<Channel> {
    pub fn new(channel: Channel, clock_hz: u32) -> Self {
        let ratio = clock_hz as f32 / 1e9;
        println!("Ratio: {ratio}");
        let bits = [
            PulseCode {
                level1: true,
                length1: ((ratio * 350.) as u32).nanos(),
                level2: false,
                length2: ((ratio * 900.) as u32).nanos(),
            },
            PulseCode {
                level1: true,
                length1: ((ratio * 600.) as u32).nanos(),
                level2: false,
                length2: ((ratio * 650.) as u32).nanos(),
            },
        ];
        Self { channel, bits }
    }

    pub fn send_one_color(&mut self, c: Srgb<u8>) -> Result<(), TransmissionError> {
        let pulses = color_to_pulse_code(c, &self.bits);
        println!("Sending pulses: {pulses:#?}");
        self.channel
            .send_pulse_sequence(RepeatMode::SingleShot, &pulses)?;
        println!("Sent pulses");
        Ok(())
    }

    pub fn send_colors(&mut self, colors: &[Srgb<u8>]) -> Result<(), TransmissionError> {
        for c in colors {
            self.send_one_color(c.clone())?;
        }
        Ok(())
    }
}

fn color_to_pulse_code(c: Srgb<u8>, bits: &[PulseCode; 2]) -> [PulseCode; 24] {
    let c: Packed<Argb, u32> = c.into();
    let mut pulses = [bits[0]; 24];
    (0..24).rev().for_each(|i| {
        let pulse: usize = ((c.color >> i) & 1) as usize;
        // println!("bit {i} {pulse} {:?}", bits[pulse]);
        pulses[i] = bits[pulse];
    });

    pulses
}
