//! Keyboard and gamepad.
//!
//! - Steer: Left/Right or A/D, or the left stick / d-pad.
//! - Gas: Up or W, or A (south) on a pad. Brake and reverse: Down or S, or B.
//! - Drift: Space or Shift (hold), or the right shoulder.
//! - Item: X, E or Ctrl, or X/Y or the left shoulder on a pad. Hold brake
//!   while using a present to drop it behind you.
//!
//! With the kitty keyboard protocol presses and releases are exact, so gas,
//! steering and drift can all be held at once. Without it a terminal only
//! auto-repeats the last key pressed, so holding gas while steering is
//! impossible; there the gas latches (Up starts it, Down stops it) and
//! Space toggles a drift, while steering counts as held while repeats keep
//! arriving.

use crate::kart::Controls;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};
use std::time::{Duration, Instant};

const PRE_REPEAT: Duration = Duration::from_millis(500);
const REPEAT_GAP: Duration = Duration::from_millis(110);
#[cfg(feature = "gamepad")]
const STICK_DEADZONE: f32 = 0.15;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Key {
    Left,
    Right,
    Gas,
    Brake,
    Drift,
}
const KEYS: usize = 5;

#[derive(Clone, Copy, Default)]
struct Hold {
    down: bool,
    last: Option<Instant>,
    repeats: u32,
}

impl Hold {
    fn held(&self, exact: bool, now: Instant) -> bool {
        if exact {
            return self.down;
        }
        let Some(last) = self.last else { return false };
        let window = if self.repeats == 0 { PRE_REPEAT } else { REPEAT_GAP };
        now.duration_since(last) <= window
    }
}

/// Menu and system commands, as edges.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Confirm,
    Back,
    Pause,
    Restart,
    Help,
    Exit,
    Left,
    Right,
    Up,
    Down,
}

#[cfg(feature = "gamepad")]
fn usable_pad(p: &gilrs::Gamepad) -> bool {
    p.mapping_source() == gilrs::MappingSource::SdlMappings || std::env::var("SLEIGHRIDE_ANY_PAD").is_ok_and(|v| v == "1")
}

#[derive(Default, Clone, Copy)]
struct Pad {
    steer: f32,
    left: bool,
    right: bool,
    gas: bool,
    brake: bool,
    drift: bool,
}

pub struct Input {
    pub exact: bool,
    holds: [Hold; KEYS],
    /// Without exact keys: the gas is latched on.
    gas_latch: bool,
    /// Without exact keys: a drift toggled on.
    drift_latch: bool,
    /// Use the item: an edge, taken by `controls`.
    item_edge: bool,
    pub cmds: Vec<Cmd>,
    #[cfg(feature = "gamepad")]
    gilrs: Option<gilrs::Gilrs>,
    pub pad_name: Option<String>,
    pad: Pad,
}

impl Input {
    pub fn new(exact: bool) -> Input {
        Input {
            exact,
            holds: [Hold::default(); KEYS],
            gas_latch: false,
            drift_latch: false,
            item_edge: false,
            cmds: Vec::new(),
            #[cfg(feature = "gamepad")]
            gilrs: None,
            pad_name: None,
            pad: Pad::default(),
        }
    }

    pub fn on_key(&mut self, k: KeyEvent, now: Instant) {
        let press = k.kind != KeyEventKind::Release;
        let fresh = k.kind == KeyEventKind::Press;
        let held = match k.code {
            KeyCode::Left | KeyCode::Char('a' | 'A') => Some(Key::Left),
            KeyCode::Right | KeyCode::Char('d' | 'D') => Some(Key::Right),
            KeyCode::Up | KeyCode::Char('w' | 'W') => Some(Key::Gas),
            KeyCode::Down | KeyCode::Char('s' | 'S') => Some(Key::Brake),
            KeyCode::Char(' ') | KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) => Some(Key::Drift),
            _ => None,
        };
        if let Some(key) = held {
            let h = &mut self.holds[key as usize];
            if self.exact {
                h.down = press;
            } else {
                // without release events every repeat arrives as a press
                if h.held(false, now) {
                    h.repeats += 1;
                } else {
                    h.repeats = 0;
                    match key {
                        Key::Gas => self.gas_latch = true,
                        Key::Brake => self.gas_latch = false,
                        Key::Drift => self.drift_latch = !self.drift_latch,
                        _ => {}
                    }
                }
                h.last = Some(now);
            }
            if fresh {
                match key {
                    Key::Drift => self.cmds.push(Cmd::Confirm),
                    Key::Left => self.cmds.push(Cmd::Left),
                    Key::Right => self.cmds.push(Cmd::Right),
                    Key::Gas => self.cmds.push(Cmd::Up),
                    Key::Brake => self.cmds.push(Cmd::Down),
                }
            }
            return;
        }
        if !fresh {
            return;
        }
        match k.code {
            KeyCode::Char('x' | 'X' | 'e' | 'E') | KeyCode::Modifier(ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl) => self.item_edge = true,
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => self.cmds.push(Cmd::Exit),
            KeyCode::Enter => self.cmds.push(Cmd::Confirm),
            KeyCode::Esc => self.cmds.push(Cmd::Back),
            KeyCode::Char('p' | 'P') => self.cmds.push(Cmd::Pause),
            KeyCode::Char('r' | 'R') => self.cmds.push(Cmd::Restart),
            KeyCode::Char('q' | 'Q') => self.cmds.push(Cmd::Back),
            KeyCode::F(1) | KeyCode::Char('h' | 'H' | '?') => self.cmds.push(Cmd::Help),
            _ => {}
        }
    }

    pub fn open_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        match gilrs::Gilrs::new() {
            Ok(g) => {
                let all: Vec<_> = g.gamepads().map(|(_, p)| (p.name().to_string(), usable_pad(&p))).collect();
                log::info!("gamepads (name, used): {all:?}");
                self.pad_name = all.into_iter().find(|(_, used)| *used).map(|(n, _)| n);
                self.gilrs = Some(g);
            }
            Err(e) => log::warn!("gamepad support unavailable: {e}"),
        }
    }

    /// Drain gamepad events; call once per frame.
    pub fn poll_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        {
            use gilrs::{Axis, Button, EventType};
            let Some(g) = &mut self.gilrs else { return };
            while let Some(ev) = g.next_event() {
                let pad = g.gamepad(ev.id);
                if !usable_pad(&pad) {
                    continue;
                }
                let down = matches!(ev.event, EventType::ButtonPressed(..));
                match ev.event {
                    EventType::Connected => {
                        self.pad_name = Some(pad.name().to_string());
                        log::info!("gamepad connected: {:?}", self.pad_name);
                    }
                    EventType::Disconnected => {
                        log::info!("gamepad disconnected");
                        self.pad_name = None;
                        self.pad = Pad::default();
                    }
                    EventType::ButtonPressed(b, _) | EventType::ButtonReleased(b, _) => {
                        log::debug!("pad button {b:?} down={down}");
                        match b {
                            Button::South => self.pad.gas = down,
                            Button::East => self.pad.brake = down,
                            Button::RightTrigger | Button::RightTrigger2 => self.pad.drift = down,
                            Button::West | Button::North | Button::LeftTrigger | Button::LeftTrigger2 if down => self.item_edge = true,
                            Button::DPadLeft => self.pad.left = down,
                            Button::DPadRight => self.pad.right = down,
                            _ => {}
                        }
                        if down {
                            match b {
                                Button::South => self.cmds.push(Cmd::Confirm),
                                Button::Start => self.cmds.push(Cmd::Pause),
                                Button::Select => self.cmds.push(Cmd::Help),
                                Button::DPadLeft => self.cmds.push(Cmd::Left),
                                Button::DPadRight => self.cmds.push(Cmd::Right),
                                Button::DPadUp => self.cmds.push(Cmd::Up),
                                Button::DPadDown => self.cmds.push(Cmd::Down),
                                _ => {}
                            }
                        }
                    }
                    EventType::AxisChanged(Axis::LeftStickX, v, _) => {
                        self.pad.steer = if v.abs() < STICK_DEADZONE { 0.0 } else { v };
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.cmds.clear();
        self.gas_latch = false;
        self.drift_latch = false;
        self.item_edge = false;
    }

    pub fn controls(&mut self, now: Instant) -> Controls {
        let item = std::mem::take(&mut self.item_edge);
        let held = |k: Key| self.holds[k as usize].held(self.exact, now);
        let keys_steer = held(Key::Right) as i8 as f32 - held(Key::Left) as i8 as f32;
        let pad_dpad = self.pad.right as i8 as f32 - self.pad.left as i8 as f32;
        let steer = if keys_steer != 0.0 {
            keys_steer
        } else if pad_dpad != 0.0 {
            pad_dpad
        } else {
            self.pad.steer
        };
        let (gas, drift) = if self.exact { (held(Key::Gas), held(Key::Drift)) } else { (self.gas_latch, self.drift_latch) };
        Controls {
            steer: steer.clamp(-1.0, 1.0),
            gas: gas || self.pad.gas,
            brake: held(Key::Brake) || self.pad.brake,
            drift: drift || self.pad.drift,
            item,
        }
    }
}
