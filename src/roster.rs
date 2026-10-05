//! Who races: Santa's helpers on everything they could take, and Santa.

use crate::kart::Stats;
use crate::model::{Elf, Ride};
use crate::render::Rgb;

pub struct Who {
    pub name: &'static str,
    pub ride: Ride,
    /// What the ride is called on the title screen.
    pub ride_name: &'static str,
    pub elf: Elf,
    pub stats: Stats,
}

impl Who {
    /// The colour that marks this racer on the minimap and standings.
    pub fn colour(&self) -> Rgb {
        self.elf.hat
    }
}

const SLEIGH: Stats = Stats { top: 63.9, accel: 1.2, turn: 2.0, grip: 9.0, charge: 1.0, weight: 1.0 };
/// Quick off the mark and nimble, a little lower on top speed, light.
const REINDEER: Stats = Stats { top: 63.9, accel: 1.4, turn: 2.2, grip: 10.0, charge: 1.05, weight: 0.8 };
/// Fast but heavy, a wide line and slow to charge a drift.
const SNOWMOBILE: Stats = Stats { top: 64.2, accel: 1.0, turn: 1.8, grip: 7.5, charge: 0.95, weight: 1.25 };
/// Low and slippery: fast, light, charges drifts quickly.
const TOBOGGAN: Stats = Stats { top: 63.2, accel: 1.2, turn: 1.9, grip: 8.0, charge: 1.1, weight: 0.7 };
/// Narrow and twitchy: the highest top speed, turns sharply, grips little.
const SKIS: Stats = Stats { top: 65.2, accel: 1.1, turn: 2.3, grip: 6.5, charge: 1.0, weight: 0.6 };
/// Santa: much faster than anyone, and nothing moves him.
pub const PLOUGH: Stats = Stats { top: 80.0, accel: 1.6, turn: 2.6, grip: 12.0, charge: 0.0, weight: 4.0 };

pub const ROSTER: [Who; 6] = [
    Who {
        name: "Bob",
        ride: Ride::Sleigh,
        ride_name: "the big sleigh",
        elf: Elf { hat: (40, 168, 70), coat: (40, 168, 70), hair: (150, 74, 34), paint: (214, 38, 46) },
        stats: SLEIGH,
    },
    Who {
        name: "Holly",
        ride: Ride::Reindeer,
        ride_name: "a reindeer",
        elf: Elf { hat: (206, 40, 56), coat: (206, 40, 56), hair: (234, 204, 112), paint: (60, 110, 220) },
        stats: REINDEER,
    },
    Who {
        name: "Sprocket",
        ride: Ride::Snowmobile,
        ride_name: "the workshop snowmobile",
        elf: Elf { hat: (52, 102, 224), coat: (52, 102, 224), hair: (44, 36, 32), paint: (240, 122, 30) },
        stats: SNOWMOBILE,
    },
    Who {
        name: "Pip",
        ride: Ride::Toboggan,
        ride_name: "a toboggan",
        elf: Elf { hat: (146, 64, 206), coat: (146, 64, 206), hair: (206, 96, 44), paint: (40, 172, 172) },
        stats: TOBOGGAN,
    },
    Who {
        name: "Juniper",
        ride: Ride::Reindeer,
        ride_name: "a reindeer",
        elf: Elf { hat: (30, 164, 150), coat: (30, 164, 150), hair: (84, 52, 34), paint: (214, 64, 144) },
        stats: REINDEER,
    },
    Who {
        name: "Nutmeg",
        ride: Ride::Skis,
        ride_name: "racing skis",
        elf: Elf { hat: (244, 200, 40), coat: (230, 176, 30), hair: (236, 236, 240), paint: (40, 160, 80) },
        stats: SKIS,
    },
];

pub const SANTA_COLOUR: Rgb = (255, 60, 60);
