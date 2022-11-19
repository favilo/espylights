use esp32c3_hal::{
    prelude::*,
    pulse_control::{ConfiguredChannel, PulseCode, RepeatMode, TransmissionError},
};
use lazy_static::lazy_static;
use palette::{Packed, Srgb};

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
}

impl<Channel: ConfiguredChannel> Ws281X<Channel> {
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }

    pub fn send_one_color(&mut self, c: Srgb<u8>) -> Result<(), TransmissionError> {
        let pulses = color_to_pulse_code(c);
        self.channel
            .send_pulse_sequence(RepeatMode::SingleShot, &pulses)
    }

    pub fn send_colors(&mut self, colors: &[Srgb<u8>]) -> Result<(), TransmissionError> {
        for c in colors {
            self.send_one_color(c.clone())?;
        }
        Ok(())
    }
}

fn color_to_pulse_code(c: Srgb<u8>) -> [PulseCode; 24] {
    let c: Packed = c.into();
    let mut pulses = [BITS[0]; 24];
    (0..24).for_each(|i| {
        let pulse: usize = (c.color >> (23 - i) & 1) as usize;
        pulses[i] = BITS[pulse];
    });

    pulses
}
