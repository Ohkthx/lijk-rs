use std::collections::HashMap;
use std::time::Instant;

use sdl3::Sdl;
use sdl3::pixels::Color;
use sdl3::rect::Rect;
use sdl3::render::{Canvas, FPoint};
use sdl3::video::Window;

use crate::client::input::{Input, InputState};
use crate::error::AppError;
use crate::net::PacketLabel;
use crate::net::Socket;
use crate::shared::payload::{Connect, Movement, PayloadId, Position, ServerState};
use crate::utils::decode;
use crate::vec2f::Vec2f;

use super::socket::ClientSocket;

/// Core of the client application.
pub struct ClientCore {
    socket: ClientSocket,   // Socket to the server.
    sdl: Sdl,               // SDL context.
    canvas: Canvas<Window>, // Canvas to draw on.
}

impl ClientCore {
    const SIZE: u16 = 32;
    const WIDTH: u32 = Self::SIZE as u32 * 20;
    const HEIGHT: u32 = Self::WIDTH;

    /// Creates a new client core by initializing the SDL context and creating a window.
    pub fn new(socket: Socket) -> Result<Self, AppError> {
        let sdl = sdl3::init().map_err(AppError::Sdl)?;
        let video = sdl.video().map_err(AppError::Sdl)?;

        // Ensure no VSYNC.
        sdl3::hint::set(sdl3::hint::names::RENDER_VSYNC, "0");
        let _ = video.gl_set_swap_interval(sdl3::video::SwapInterval::Immediate);

        let window = video
            .window("LIJK", Self::WIDTH, Self::HEIGHT)
            .build()
            .map_err(|why| AppError::Window(why.to_string()))?;

        let canvas = window.into_canvas();

        Ok(Self {
            socket: ClientSocket::new(socket),
            sdl,
            canvas,
        })
    }

    /// Runs the main loop for the client application. Handles input events, server updates, and rendering.
    pub fn run(&mut self) -> Result<(), AppError> {
        const LERP_SNAP_SPEED: f32 = 10.0; // “pull‑to‑server” speed in Hz

        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        self.canvas.present();
        let mut event_pump = self.sdl.event_pump().map_err(AppError::Sdl)?;

        // Wait for the connection.
        self.socket.wait_for_connection()?;

        let mut entity_id = 0;

        let mut dest: Vec2f = Vec2f::ZERO;
        let mut speed: u8 = 1;

        let mut last_frame_time = Instant::now();
        let mut input_state = InputState::new();

        // Represents the server state.
        let mut server_state = ServerState { tps: 0, tick_id: 0 };
        let mut server_state_ms = Instant::now(); // Time when the server state was last received.
        let mut _server_tick_est: u64; // Estimated tick from the server.

        let mut entity_pos: HashMap<u32, (Vec2f, Vec2f, Vec2f)> = HashMap::new();

        'game_loop: loop {
            // Get the delta time.
            let now = Instant::now();
            let dt = (now - last_frame_time).as_secs_f32();
            last_frame_time = now;

            // Calculate the server tick based on the server state and the elapsed time.
            let tick_duration = 1.0 / f32::from(server_state.tps);
            let elapsed = now - server_state_ms;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let ticks = (elapsed.as_secs_f32() / tick_duration).floor() as u64;
            _server_tick_est = server_state.tick_id + ticks;

            // Process the packets from the server.
            let packets = self.socket.run_step()?;
            for packet in packets {
                match packet.label() {
                    PacketLabel::Extension(id) if id == u8::from(PayloadId::Connect) => {
                        let Connect(entity, spawn_point) = decode::<Connect>(&packet)?;
                        entity_id = entity;
                        entity_pos.insert(entity, (spawn_point, spawn_point, Vec2f::ZERO));
                        dest = spawn_point;
                    }
                    PacketLabel::Extension(id) if id == u8::from(PayloadId::State) => {
                        server_state = decode::<ServerState>(&packet)?;
                        _server_tick_est = server_state.tick_id;
                        server_state_ms = Instant::now(); // Reset the server state time.
                    }
                    PacketLabel::Extension(id) if id == u8::from(PayloadId::Position) => {
                        let Position(entity, server_pos, vel) = decode::<Position>(&packet)?;
                        let scaled_pos = server_pos.scale(f32::from(Self::SIZE));
                        let scaled_view = vel.scale(f32::from(Self::SIZE));
                        if let Some((_local, remote, view)) = entity_pos.get_mut(&entity) {
                            *remote = scaled_pos;
                            *view = scaled_view;
                        } else {
                            // Add a new remote player.
                            entity_pos.insert(entity, (scaled_pos, scaled_pos, scaled_view));
                        }
                    }

                    _ => {}
                }
            }

            let mut move_delta = Vec2f::ZERO; // Reset the movement delta.
            input_state.get_input(&mut event_pump, self.canvas.window().id());
            for input in &input_state.events {
                match input {
                    Input::Quit => break 'game_loop,
                    Input::Cursor(dx, dy) => dest = Vec2f(*dx, *dy),
                    Input::Speed(s) => speed = *s,
                    Input::MoveDelta(delta) => {
                        if let Some((local, _, _)) = entity_pos.get_mut(&entity_id) {
                            if dest.length() > f32::from(Self::SIZE) {
                                dest = *local;
                            }

                            move_delta = *delta;
                            dest += delta.scale(f32::from(Self::SIZE));
                            *local += delta.scale(dt * f32::from(speed) * f32::from(Self::SIZE));
                        }
                    }
                }
            }

            // If the movement delta is not zero or if the movement keys have been released,
            if move_delta != Vec2f::ZERO
                || (input_state.is_movement_released() && !input_state.is_movement_held())
            {
                // Send the movement to the server.
                let payload = Movement(move_delta, speed);
                self.socket.send(
                    PacketLabel::Extension(u8::from(PayloadId::Movement)),
                    Some(payload),
                )?;
            }

            self.canvas.set_draw_color(Color::RGB(255, 255, 255));
            self.canvas.clear();

            // Draw the grid and the player.
            self.draw_grid(Color::RGB(0, 0, 0));

            // Render the local player's position.
            for (entity, (local, remote, view)) in &mut entity_pos {
                // Update the position of the remote player.
                *local += (*remote - *local).scale((LERP_SNAP_SPEED * dt).min(1.0));

                // Render the remote players.
                if entity == &entity_id {
                    self.render_pos(*remote, Color::RGB(255, 0, 0));
                    self.render_pos(*local, Color::RGB(0, 0, 255));
                } else {
                    self.render_pos(*remote, Color::RGB(0, 255, 255));
                    self.render_pos(*local, Color::RGB(0, 255, 0));
                }

                // Render the direction they are facing.
                let start_x = remote.0 + f32::from(Self::SIZE) / 2.0;
                let start_y = remote.1 + f32::from(Self::SIZE) / 2.0;
                let start = Vec2f(start_x, start_y);
                self.render_line(start, start + view.scale(4.0), Color::RGB(255, 0, 0));
            }

            // self.render_pos(dest, Color::RGB(0, 0, 0));

            self.canvas.present();
        }

        Ok(())
    }

    /// Draws a grid on the canvas.
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn draw_grid(&mut self, color: Color) {
        self.canvas.set_draw_color(color);
        for x in (0..Self::WIDTH).step_by(Self::SIZE.into()) {
            let _ = self.canvas.draw_line(
                FPoint::new(x as f32, 0.0),
                FPoint::new(x as f32, Self::HEIGHT as f32),
            );
        }
        for y in (0..Self::HEIGHT).step_by(Self::SIZE.into()) {
            let _ = self.canvas.draw_line(
                FPoint::new(0.0, y as f32),
                FPoint::new(Self::WIDTH as f32, y as f32),
            );
        }
    }

    /// Renders a position on the canvas.
    pub(crate) fn render_pos(&mut self, pos: Vec2f, color: Color) {
        self.canvas.set_draw_color(color);
        #[allow(clippy::cast_possible_truncation)]
        let _ = self.canvas.fill_rect(Rect::new(
            (pos.0).round() as i32, // x position
            (pos.1).round() as i32, // x position
            Self::SIZE.into(),      // width
            Self::SIZE.into(),      // height
        ));
    }

    /// Renders a colored line from the starting position to ending.
    pub(crate) fn render_line(&mut self, start: Vec2f, end: Vec2f, color: Color) {
        self.canvas.set_draw_color(color);
        let _ = self.canvas.draw_line(start, end);
    }
}

impl From<Vec2f> for FPoint {
    fn from(vec: Vec2f) -> FPoint {
        FPoint { x: vec.0, y: vec.1 }
    }
}
