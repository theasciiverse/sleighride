//! Racing together over iroh: on the local network (found by a beacon, no
//! addresses to type) or across the internet (by the host's iroh id).
//!
//! The host runs the only race. Up to five guests each take one of the six
//! racers from the AI: they choose their racer, the host chooses the course
//! and difficulty and starts. Guests send their controls every step; the
//! host sends the whole race back every step. A guest who leaves mid-race
//! hands their racer back to the AI.

use crate::auto::Ai;
use crate::kart::Controls;
use crate::lan;
use crate::net::{self, ClientId, Evt, Net};
use crate::proto::{Lobby, Msg};
use crate::roster::ROSTER;

/// A guest at the host's table.
pub struct Seat {
    pub client: ClientId,
    pub name: String,
    pub pick: usize,
    input: Controls,
}

pub struct Host {
    net: Net,
    pub name: String,
    /// Our iroh id, once the endpoint is up: what internet guests dial.
    pub id: Option<String>,
    pub seats: Vec<Seat>,
    pub race_no: u32,
    lobby_t: f32,
    pub status: String,
}

pub struct Guest {
    net: Net,
    pub status: String,
    pub lobby: Option<Lobby>,
    pub seat: Option<u32>,
    /// The race number we last built.
    pub race_no: u32,
    ping_t: f32,
    pub gone: bool,
}

/// Looking for games on the local network.
pub struct Browse {
    listener: Option<lan::Listener>,
    pub found: Vec<lan::Found>,
    pub status: String,
    pub name: String,
}

pub enum Link {
    Host(Host),
    Guest(Guest),
    Browse(Browse),
}

impl Link {
    pub fn host(name: String) -> Link {
        Link::Host(Host { net: Net::host(name.clone()), name, id: None, seats: Vec::new(), race_no: 0, lobby_t: 0.0, status: "starting...".into() })
    }

    pub fn join(addr: iroh::EndpointAddr, name: String) -> Link {
        Link::Guest(Guest { net: Net::join(addr, name), status: "connecting...".into(), lobby: None, seat: None, race_no: 0, ping_t: 0.0, gone: false })
    }

    pub fn browse(name: String) -> Link {
        let (listener, status) = match lan::Listener::start(lan::PORT) {
            Ok(l) => (Some(l), "looking for races on the local network...".to_string()),
            Err(e) => (None, format!("can't listen on the local network: {e}")),
        };
        Link::Browse(Browse { listener, found: Vec::new(), status, name })
    }
}

impl Host {
    /// Racers driven by people this race: the host's first, then the seats.
    /// A seat whose pick is taken gets the first free racer.
    pub fn humans(&mut self, own: usize) -> Vec<usize> {
        let mut taken = vec![own];
        for s in &mut self.seats {
            if taken.contains(&s.pick) {
                s.pick = (0..ROSTER.len()).find(|w| !taken.contains(w)).unwrap_or(s.pick);
            }
            taken.push(s.pick);
        }
        taken
    }

    /// Everyone's controls this step: (racer index, controls).
    pub fn inputs(&self, own: (usize, Controls)) -> Vec<(usize, Controls)> {
        let mut v = vec![own];
        v.extend(self.seats.iter().map(|s| (s.pick, s.input)));
        v
    }

    pub fn lobby(&self, course: usize, level: usize, own: usize, racing: bool) -> Lobby {
        let mut players = vec![(0, self.name.clone(), own)];
        players.extend(self.seats.iter().map(|s| (s.client, s.name.clone(), s.pick)));
        Lobby { course, level, players, race_no: self.race_no, racing }
    }

    /// Network events; returns seats that left (their racers go back to the AI).
    pub fn poll(&mut self, own: usize, racing: bool) -> Vec<usize> {
        let mut left = Vec::new();
        while let Some(e) = self.net.poll() {
            if !matches!(e, Evt::ClientMsg { msg: Msg::Input(_) | Msg::Ping, .. }) {
                log::info!("net {e:?}");
            }
            match e {
                Evt::Hosting { id } => {
                    self.id = Some(id.to_string());
                    self.status = "waiting for players".into();
                }
                Evt::Failed(why) => self.status = why,
                Evt::ClientJoined { client, name } => {
                    if racing {
                        self.net.send(net::Cmd::SendTo(client, Msg::Reject { reason: "a race is on; join when it ends".into() }));
                        self.net.send(net::Cmd::Kick(client));
                        continue;
                    }
                    let mut taken: Vec<usize> = vec![own];
                    taken.extend(self.seats.iter().map(|s| s.pick));
                    let pick = (0..ROSTER.len()).find(|w| !taken.contains(w)).unwrap_or(0);
                    self.net.send(net::Cmd::SendTo(client, Msg::Welcome { seat: client }));
                    self.seats.push(Seat { client, name, pick, input: Controls::default() });
                    self.lobby_t = 1.0;
                }
                Evt::ClientMsg { client, msg } => {
                    let Some(seat) = self.seats.iter_mut().find(|s| s.client == client) else { continue };
                    match msg {
                        Msg::Input(c) => {
                            // an item press must not be lost between frames
                            let item = seat.input.item && !c.item;
                            seat.input = c;
                            seat.input.item |= item;
                        }
                        Msg::Pick(p) if !racing && p < ROSTER.len() => {
                            seat.pick = p;
                            self.lobby_t = 1.0;
                        }
                        _ => {}
                    }
                }
                Evt::ClientLeft { client } => {
                    if let Some(i) = self.seats.iter().position(|s| s.client == client) {
                        left.push(self.seats.remove(i).pick);
                        self.lobby_t = 1.0;
                    }
                }
                _ => {}
            }
        }
        left
    }

    /// The item press was taken by this step.
    pub fn used_inputs(&mut self) {
        for s in &mut self.seats {
            s.input.item = false;
        }
    }

    pub fn send_lobby(&mut self, dt: f32, lobby: Lobby) {
        self.lobby_t += dt;
        if self.lobby_t >= 0.5 {
            self.lobby_t = 0.0;
            self.net.send(net::Cmd::Beacon { players: 1 + self.seats.len() as u8, in_game: lobby.racing });
            self.net.send(net::Cmd::Broadcast(Msg::Lobby(lobby)));
        }
    }

    pub fn send_frame(&self, snap: crate::race::Snap) {
        if !self.seats.is_empty() {
            self.net.send(net::Cmd::Broadcast(Msg::Frame { race_no: self.race_no, snap: Box::new(snap) }));
        }
    }
}

/// What a guest heard this step.
pub enum Heard {
    Lobby(Lobby),
    Frame(u32, Box<crate::race::Snap>),
}

impl Guest {
    pub fn poll(&mut self) -> Vec<Heard> {
        let mut out = Vec::new();
        while let Some(e) = self.net.poll() {
            if !matches!(e, Evt::HostMsg(Msg::Frame { .. } | Msg::Lobby(_))) {
                log::info!("net {e:?}");
            }
            match e {
                Evt::Connected => self.status = "connected, waiting for the host".into(),
                Evt::Failed(why) => {
                    self.status = why;
                    self.gone = true;
                }
                Evt::Disconnected => {
                    self.status = "the host has gone".into();
                    self.gone = true;
                }
                Evt::HostMsg(Msg::Welcome { seat }) => self.seat = Some(seat),
                Evt::HostMsg(Msg::Reject { reason }) => {
                    self.status = reason;
                    self.gone = true;
                }
                Evt::HostMsg(Msg::Lobby(l)) => {
                    self.lobby = Some(l.clone());
                    out.push(Heard::Lobby(l));
                }
                Evt::HostMsg(Msg::Frame { race_no, snap }) => out.push(Heard::Frame(race_no, snap)),
                _ => {}
            }
        }
        out
    }

    /// My racer, as the host sees it.
    pub fn my_pick(&self) -> Option<usize> {
        let (l, seat) = (self.lobby.as_ref()?, self.seat?);
        l.players.iter().find(|p| p.0 == seat).map(|p| p.2)
    }

    pub fn send_input(&mut self, c: Controls) {
        self.net.send(net::Cmd::Broadcast(Msg::Input(c)));
    }

    pub fn send_pick(&mut self, p: usize) {
        self.net.send(net::Cmd::Broadcast(Msg::Pick(p)));
    }

    pub fn keep_alive(&mut self, dt: f32) {
        self.ping_t += dt;
        if self.ping_t >= 2.0 {
            self.ping_t = 0.0;
            self.net.send(net::Cmd::Broadcast(Msg::Ping));
        }
    }
}

impl Browse {
    pub fn refresh(&mut self) {
        if let Some(l) = &self.listener {
            self.found = l.hosts();
        }
    }
}

/// A seat left mid-race: their racer drives itself from here.
pub fn hand_to_ai(race: &mut crate::race::Race, who: usize, seed: u32) {
    if let Some(r) = race.racers.get_mut(who) {
        if r.ai.is_none() {
            r.ai = Some(Ai::new(who, seed));
        }
    }
}
