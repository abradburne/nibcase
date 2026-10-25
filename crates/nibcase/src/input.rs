//! One gamepad snapshot; Menu and Back are frontend controls and never reach libretro.
use nibcase_libretro::ffi::*;
use nibkit::input::{Button as Key, Face};
use sdl2::controller::{Axis, Button};
use sdl2::event::Event;

pub enum Action {
    Press(Key),
    Release(Key),
    Quit,
    Touch {
        window: u32,
        x: f32,
        y: f32,
        normalized: bool,
    },
}
#[derive(Default)]
pub struct Input {
    digital: u16,
    pub analog: [i16; 2],
    pub guide: bool,
    pub chord: bool,
    pub fast: bool,
}
fn bit(key: Key) -> Option<u32> {
    Some(match key {
        Key::B => RETRO_DEVICE_ID_JOYPAD_B,
        Key::Y => RETRO_DEVICE_ID_JOYPAD_Y,
        Key::Select => RETRO_DEVICE_ID_JOYPAD_SELECT,
        Key::Start => RETRO_DEVICE_ID_JOYPAD_START,
        Key::Up => RETRO_DEVICE_ID_JOYPAD_UP,
        Key::Down => RETRO_DEVICE_ID_JOYPAD_DOWN,
        Key::Left => RETRO_DEVICE_ID_JOYPAD_LEFT,
        Key::Right => RETRO_DEVICE_ID_JOYPAD_RIGHT,
        Key::A => RETRO_DEVICE_ID_JOYPAD_A,
        Key::X => RETRO_DEVICE_ID_JOYPAD_X,
        Key::L => RETRO_DEVICE_ID_JOYPAD_L,
        Key::R => RETRO_DEVICE_ID_JOYPAD_R,
        _ => return None,
    })
}
fn pad(button: Button) -> Option<Key> {
    Some(match button {
        Button::A | Button::B | Button::X | Button::Y => {
            let f = match button {
                Button::A => Face::A,
                Button::B => Face::B,
                Button::X => Face::X,
                _ => Face::Y,
            };
            match nibkit::input::printed(f) {
                Face::A => Key::A,
                Face::B => Key::B,
                Face::X => Key::X,
                Face::Y => Key::Y,
            }
        }
        Button::DPadUp => Key::Up,
        Button::DPadDown => Key::Down,
        Button::DPadLeft => Key::Left,
        Button::DPadRight => Key::Right,
        Button::LeftShoulder => Key::L,
        Button::RightShoulder => Key::R,
        Button::Start => Key::Start,
        Button::Back => Key::Select,
        Button::Guide => Key::Menu,
        _ => return None,
    })
}
impl Input {
    pub fn clear(&mut self) {
        self.digital = 0;
    }
    pub fn buttons(&self, stick: bool) -> u16 {
        if self.guide {
            return 0;
        }
        let mut value = self.digital;
        if stick {
            for (axis, negative, positive) in [
                (0, RETRO_DEVICE_ID_JOYPAD_LEFT, RETRO_DEVICE_ID_JOYPAD_RIGHT),
                (1, RETRO_DEVICE_ID_JOYPAD_UP, RETRO_DEVICE_ID_JOYPAD_DOWN),
            ] {
                if self.analog[axis] < -12000 {
                    value |= 1 << negative;
                } else if self.analog[axis] > 12000 {
                    value |= 1 << positive;
                }
            }
        }
        value
    }
    pub fn event(&mut self, event: Event) -> Option<Action> {
        let (key, down) = match event {
            Event::Quit { .. } => return Some(Action::Quit),
            Event::KeyDown {
                keycode: Some(code),
                repeat: false,
                ..
            }
            | Event::KeyUp {
                keycode: Some(code),
                ..
            } => {
                let down = matches!(event, Event::KeyDown { .. });
                let name = code.name();
                let key = if nibkit::input::BACK_KEYCODES.contains(&name.as_str()) {
                    Key::Back
                } else {
                    nibkit::input::keyboard(&name)?
                };
                (key, down)
            }
            Event::ControllerButtonDown { button, .. } => (pad(button)?, true),
            Event::ControllerButtonUp { button, .. } => (pad(button)?, false),
            Event::ControllerAxisMotion { axis, value, .. } => {
                let index = match axis {
                    Axis::LeftX => 0,
                    Axis::LeftY => 1,
                    _ => return None,
                };
                self.analog[index] = if value.unsigned_abs() < 4000 {
                    0
                } else {
                    value
                };
                return None;
            }
            Event::FingerDown { x, y, .. } => {
                return Some(Action::Touch {
                    window: 0,
                    x,
                    y,
                    normalized: true,
                })
            }
            Event::MouseButtonDown {
                window_id, x, y, ..
            } if !nibkit::on_device() => {
                return Some(Action::Touch {
                    window: window_id,
                    x: x as f32,
                    y: y as f32,
                    normalized: false,
                })
            }
            _ => return None,
        };
        if let Some(bit) = bit(key) {
            if down && !self.guide {
                self.digital |= 1 << bit;
            } else {
                self.digital &= !(1 << bit);
            }
        }
        Some(if down {
            Action::Press(key)
        } else {
            Action::Release(key)
        })
    }
}
pub fn menu(key: Key) -> Option<nibkit::menu::Key> {
    use nibkit::menu::Key as M;
    Some(match key {
        Key::Up => M::Up,
        Key::Down => M::Down,
        Key::Left => M::Left,
        Key::Right => M::Right,
        Key::A => M::A,
        Key::B => M::B,
        Key::X => M::X,
        Key::Y => M::Y,
        Key::L => M::L,
        Key::R => M::R,
        _ => return None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn printed_buttons_and_reserved_controls() {
        let mut i = Input::default();
        i.event(Event::ControllerButtonDown {
            timestamp: 0,
            which: 0,
            button: Button::B,
        });
        assert_eq!(i.buttons(false), 1 << RETRO_DEVICE_ID_JOYPAD_A);
        i.guide = true;
        assert_eq!(i.buttons(false), 0);
        assert!(bit(Key::Menu).is_none());
        assert!(bit(Key::Back).is_none());
    }
}
