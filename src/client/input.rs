use std::collections::HashSet;

use sdl3::EventPump;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::mouse::MouseButton;

use crate::vec2f::Vec2f;

/// Represents the input from the user.
pub(crate) enum Input {
    Quit,             // The user has requested to quit the application.
    Cursor(f32, f32), // The cursor position.
    MoveDelta(Vec2f), // The delta movement.
    Speed(u8),        // The speed of the player.
}

/// Represents the state of the input.
pub(crate) struct InputState {
    held: HashSet<Keycode>,     // The keys that are currently held down.
    released: HashSet<Keycode>, // The keys that have been released.
    pub events: Vec<Input>,     // The events that have been triggered.
}

impl InputState {
    /// Creates a new instance of the input state.
    pub fn new() -> Self {
        Self {
            held: HashSet::new(),
            released: HashSet::new(),
            events: Vec::new(),
        }
    }

    /// Checks if any movement keys are currently held down.
    pub fn is_movement_held(&self) -> bool {
        self.held.contains(&Keycode::W)
            || self.held.contains(&Keycode::A)
            || self.held.contains(&Keycode::S)
            || self.held.contains(&Keycode::D)
    }

    /// Checks if any movement keys have been released.
    pub fn is_movement_released(&self) -> bool {
        self.released.contains(&Keycode::W)
            || self.released.contains(&Keycode::A)
            || self.released.contains(&Keycode::S)
            || self.released.contains(&Keycode::D)
    }

    /// Obtains the input from the user.
    pub fn get_input(&mut self, pump: &mut EventPump, win_id: u32) {
        let mut delta = Vec2f(0.0, 0.0);
        let mut last_pos = Vec2f(f32::MIN, f32::MIN);
        let mut last_speed = 0u8;
        self.events = Vec::new();
        self.released.clear();

        for event in pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.events = vec![Input::Quit];
                    return;
                }

                Event::MouseButtonDown {
                    x,
                    y,
                    window_id,
                    mouse_btn: MouseButton::Left,
                    ..
                } if window_id == win_id => {
                    last_pos = Vec2f(x, y);
                }

                Event::KeyDown {
                    keycode: Some(keycode),
                    window_id,
                    repeat: false,
                    ..
                } if window_id == win_id => {
                    self.released.remove(&keycode);
                    self.held.insert(keycode);
                }

                Event::KeyUp {
                    keycode: Some(keycode),
                    window_id,
                    ..
                } if window_id == win_id => {
                    self.held.remove(&keycode);
                    self.released.insert(keycode);
                }

                _ => (),
            }
        }

        if last_pos != Vec2f(f32::MIN, f32::MIN) {
            self.events.push(Input::Cursor(last_pos.0, last_pos.1));
        }

        // Accumulate the delta movement.
        for keycode in &self.held {
            match keycode {
                Keycode::W => delta += Vec2f(0.0, -1.0), // Move up
                Keycode::A => delta += Vec2f(-1.0, 0.0), // Move left
                Keycode::S => delta += Vec2f(0.0, 1.0),  // Move down
                Keycode::D => delta += Vec2f(1.0, 0.0),  // Move right
                Keycode::_1 => last_speed = 1,
                Keycode::_2 => last_speed = 2,
                Keycode::_3 => last_speed = 3,
                _ => (),
            }
        }

        if delta != Vec2f(0.0, 0.0) {
            if delta.length() > 1.0 {
                delta = delta.normalized();
            }

            self.events.push(Input::MoveDelta(delta));
        }

        if last_speed != 0 {
            self.events.push(Input::Speed(last_speed));
        }
    }
}
