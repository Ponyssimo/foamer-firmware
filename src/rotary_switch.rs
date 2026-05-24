use defmt::Format;
use embassy_rp::{
    Peri,
    gpio::{Input, Pin, Pull},
};

#[derive(Format)]
pub struct RotarySwitch<'a> {
    pub pins: [Input<'a>; 4],
}

impl<'a> RotarySwitch<'a> {
    pub fn new(
        pin_0: Peri<'a, impl Pin>,
        pin_1: Peri<'a, impl Pin>,
        pin_2: Peri<'a, impl Pin>,
        pin_3: Peri<'a, impl Pin>,
    ) -> Self {
        Self {
            pins: [
                Input::new(pin_0, Pull::Up),
                Input::new(pin_1, Pull::Up),
                Input::new(pin_2, Pull::Up),
                Input::new(pin_3, Pull::Up),
            ],
        }
    }

    pub fn read(&mut self) -> Option<u8> {
        let mut value = 0;
        for (index, pin) in self.pins.iter().enumerate() {
            value |= (pin.is_low() as u8) << index;
        }
        if value < 10 {
            Some(value)
        } else {
            defmt::warn!(
                "Did you buy a fancier rotary switch? I thought we only had 10 positions... Value was {}",
                value
            );
            None
        }
    }
}
