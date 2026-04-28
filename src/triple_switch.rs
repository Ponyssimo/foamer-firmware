use defmt::Format;
use embassy_rp::{
    Peri,
    gpio::{Flex, Level, Pin, Pull},
};
use embassy_time::{Duration, Timer};

#[derive(Format)]
pub struct TripleSwitch<'a> {
    pin: Flex<'a>,
}
#[derive(Format, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum TripleSwitchState {
    Up,
    Middle,
    Down,
}
impl<'a> TripleSwitch<'a> {
    pub fn new(pin: Peri<'a, impl Pin>) -> Self {
        Self {
            pin: Flex::new(pin),
        }
    }

    pub async fn read(&mut self) -> TripleSwitchState {
        self.pin.set_pull(Pull::Up);
        Timer::after(Duration::from_micros(100)).await;
        let up = self.pin.get_level();
        self.pin.set_pull(Pull::Down);
        Timer::after(Duration::from_micros(100)).await;
        let down = self.pin.get_level();
        match (up, down) {
            (Level::High, Level::High) => TripleSwitchState::Up,
            (Level::Low, Level::Low) => TripleSwitchState::Down,
            // Follows our pulls, probably floating
            (Level::High, Level::Low) => TripleSwitchState::Middle,

            (Level::Low, Level::High) => {
                defmt::error!(
                    "Somehow we read the opposite of our pulls? Did the user just change the state during our reads? I guess we'll call it Middle?"
                );
                TripleSwitchState::Middle
            }
        }
    }
}
