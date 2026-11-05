// Deepbore Quarry's rules: no game type here, so they build and test alone.
// The same file is the Cargo library and `game::deepbore_quarry` in the game.

use serde::{Deserialize, Serialize};

pub const ID: &str = "deepbore_quarry_native";
pub const VERSION: &str = "0.1.0";
/// The folder in the world's folder.
pub const DIR: &str = "horizon-deepbore-quarry";
pub const BLOCK: &str = "deepbore_quarry:quarry";
pub const MENU: &str = "deepbore_quarry:quarry";
/// The container: the fuel slot, the upgrade slots, then slots the screen
/// hides, where a broken block's drops wait for room in the chest. The engine
/// saves them and drops them with the block.
pub const SLOTS: usize = FIRST_BUFFER + BUFFER_SLOTS;
pub const FUEL_SLOT: usize = 0;
pub const FIRST_UPGRADE: usize = 1;
pub const FIRST_BUFFER: usize = 5;
pub const BUFFER_SLOTS: usize = 9;

// ---- upgrades --------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upgrade {
    Area,
    Speed,
    Fluids,
    Chunk,
}

impl Upgrade {
    /// How many of it one quarry takes: a second fluids or chunk upgrade would do nothing.
    fn limit(self) -> usize {
        match self {
            Upgrade::Area | Upgrade::Speed => 3,
            Upgrade::Fluids | Upgrade::Chunk => 1,
        }
    }
}

pub const UPGRADE_SLOTS: usize = 4;

/// What a quarry's upgrade slots add up to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Upgrades {
    pub area: u8,
    pub speed: u8,
    pub fluids: bool,
    pub chunk: bool,
}

impl Upgrades {
    pub fn from_slots(slots: &[Option<Upgrade>; UPGRADE_SLOTS]) -> Upgrades {
        // The menu's slot rule sees one stack, not the others, so a slot can hold
        // one over the limit: that one adds nothing.
        let count = |kind: Upgrade| slots.iter().filter(|s| **s == Some(kind)).count().min(kind.limit());
        Upgrades {
            area: count(Upgrade::Area) as u8,
            speed: count(Upgrade::Speed) as u8,
            fluids: count(Upgrade::Fluids) > 0,
            chunk: count(Upgrade::Chunk) > 0,
        }
    }
}

/// Whether `new` may go in upgrade slot `slot`, whatever that slot holds now.
pub fn accepts(slots: &[Option<Upgrade>; UPGRADE_SLOTS], slot: usize, new: Upgrade) -> bool {
    let others = slots
        .iter()
        .enumerate()
        .filter(|(i, s)| *i != slot && **s == Some(new))
        .count();
    others < new.limit()
}

/// The side of the dug square.
pub fn side(area: u8) -> i32 {
    5 + 4 * i32::from(area)
}

/// Game ticks between two broken blocks.
pub fn ticks_per_block(speed: u8) -> u32 {
    40 >> speed
}

/// Burn time one block costs: a coal's 1600 ticks dig eight.
pub const BURN_PER_BLOCK: i32 = 200;

pub fn upgrade_of(item: &str) -> Option<Upgrade> {
    match item {
        "deepbore_quarry:area_upgrade" => Some(Upgrade::Area),
        "deepbore_quarry:speed_upgrade" => Some(Upgrade::Speed),
        "deepbore_quarry:fluids_upgrade" => Some(Upgrade::Fluids),
        "deepbore_quarry:chunk_upgrade" => Some(Upgrade::Chunk),
        _ => None,
    }
}

// ---- the dug area ----------------------------------------------------------

/// The way the quarry digs: north is -z, east is +x.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Facing {
    North,
    South,
    West,
    East,
}

impl Facing {
    /// The way to dig from the block's `facing`, the side towards the player
    /// who placed it: on, past the block, the way they were looking.
    pub fn away_from(block_facing: &str) -> Option<Facing> {
        match block_facing {
            "north" => Some(Facing::South),
            "south" => Some(Facing::North),
            "west" => Some(Facing::East),
            "east" => Some(Facing::West),
            _ => None,
        }
    }
}

/// `from` moved up to `by` straight towards `to`, never past it.
pub fn approach(from: [f32; 3], to: [f32; 3], by: f32) -> [f32; 3] {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len <= by {
        return to;
    }
    let f = by / len;
    [from[0] + d[0] * f, from[1] + d[1] * f, from[2] + d[2] * f]
}

/// The square in front of the quarry, `[x, z]`, both ends included, from the
/// quarry's own level down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub min: [i32; 2],
    pub max: [i32; 2],
    pub top: i32,
}

pub fn area(pos: [i32; 3], facing: Facing, side: i32) -> Area {
    let [x, y, z] = pos;
    let half = side / 2;
    let (min, max) = match facing {
        Facing::North => ([x - half, z - side], [x + half, z - 1]),
        Facing::South => ([x - half, z + 1], [x + half, z + side]),
        Facing::West => ([x - side, z - half], [x - 1, z + half]),
        Facing::East => ([x + 1, z - half], [x + side, z + half]),
    };
    Area { min, max, top: y }
}

/// Where the quarry is: a layer, and a cell of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub y: i32,
    pub index: u32,
}

impl Area {
    fn side(&self) -> i32 {
        self.max[0] - self.min[0] + 1
    }
    pub fn start(&self) -> Cursor {
        Cursor { y: self.top, index: 0 }
    }
    pub fn pos(&self, c: Cursor) -> [i32; 3] {
        let side = self.side();
        let index = c.index as i32;
        [self.min[0] + index % side, c.y, self.min[1] + index / side]
    }
    /// The cell after `c`, row by row and then a layer down; none once the
    /// world's bottom layer `min_y` is done.
    pub fn next(&self, c: Cursor, min_y: i32) -> Option<Cursor> {
        if c.y < min_y {
            return None;
        }
        let side = self.side() as u32;
        if c.index + 1 < side * side {
            return Some(Cursor { y: c.y, index: c.index + 1 });
        }
        (c.y > min_y).then_some(Cursor { y: c.y - 1, index: 0 })
    }
    #[cfg(test)]
    pub fn contains(&self, pos: [i32; 3]) -> bool {
        (self.min[0]..=self.max[0]).contains(&pos[0])
            && (self.min[1]..=self.max[1]).contains(&pos[2])
            && pos[1] <= self.top
    }
    /// The chunks a quarry at `quarry` needs loaded: the area's, its own and its chest's.
    pub fn chunks(&self, quarry: [i32; 3], chest: Option<[i32; 3]>) -> Vec<[i32; 2]> {
        let mut chunks = Vec::new();
        for x in (self.min[0] >> 4)..=(self.max[0] >> 4) {
            for z in (self.min[1] >> 4)..=(self.max[1] >> 4) {
                chunks.push([x, z]);
            }
        }
        for p in std::iter::once(quarry).chain(chest) {
            chunks.push([p[0] >> 4, p[2] >> 4]);
        }
        chunks.sort_unstable();
        chunks.dedup();
        chunks
    }
}

// ---- one step of the quarry ------------------------------------------------

/// What the cell under the cursor is to the quarry. `Kept` is a block with
/// contents, or a quarry or its chest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Air,
    Solid,
    Fluid,
    Unbreakable,
    Kept,
    /// Its chunk isn't loaded, so nothing is known about it.
    Unloaded,
}

/// Why a quarry stands still, in the order `status_code` numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Off,
    NoFuel,
    NoChest,
    ChestFull,
    Fluid,
    Redstone,
    TooLarge,
    Done,
    Unloaded,
}

/// A quarry's saved state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quarry {
    pub on: bool,
    /// Burn time left, in ticks of furnace fuel.
    pub burn: i32,
    /// The next cell; none once the pit is done.
    pub cursor: Option<Cursor>,
    /// Ticks until the next block.
    pub wait: u32,
    pub side: i32,
    pub facing: Facing,
    pub owner: u128,
}

impl Quarry {
    /// A quarry just placed at `pos`: switched off, with no upgrade.
    pub fn new(owner: u128, facing: Facing, pos: [i32; 3]) -> Quarry {
        let side = side(0);
        Quarry {
            on: false,
            burn: 0,
            cursor: Some(area(pos, facing, side).start()),
            wait: 0,
            side,
            facing,
            owner,
        }
    }
    /// The upgrades give another size: the pit starts over from the top, where
    /// the cells already empty cost nothing.
    pub fn resize(&mut self, pos: [i32; 3], side: i32) {
        if side != self.side {
            self.side = side;
            self.cursor = Some(area(pos, self.facing, side).start());
        }
    }
    pub fn area(&self, pos: [i32; 3]) -> Area {
        area(pos, self.facing, self.side)
    }
    pub fn target(&self, pos: [i32; 3]) -> Option<[i32; 3]> {
        self.cursor.map(|c| self.area(pos).pos(c))
    }
}

/// What the game says about a quarry and its surroundings this tick.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub upgrades: Upgrades,
    pub powered: bool,
    /// The largest side the server allows.
    pub max_side: i32,
    /// The world's bottom layer.
    pub min_y: i32,
    pub chest: bool,
    /// It holds items the chest had no room for.
    pub pending: bool,
    /// The burn time of the item in the fuel slot.
    pub fuel: Option<i32>,
    pub cell: Cell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Stopped(Stop),
    /// Between two blocks.
    Wait,
    /// The cell cost nothing: step again.
    Skip,
    /// Take an item from the fuel slot, add its burn time to `burn`, and step again.
    Burn,
    Break { pos: [i32; 3], fluid: bool },
}

/// How many free cells one tick may pass, so an empty layer takes a moment
/// and not a frozen server.
pub const SKIPS_PER_TICK: usize = 64;

pub fn step(q: &mut Quarry, pos: [i32; 3], input: &Input) -> Action {
    use Action::Stopped;
    if !q.on {
        return Stopped(Stop::Off);
    }
    let Some(cursor) = q.cursor else {
        return Stopped(Stop::Done);
    };
    if cursor.y < input.min_y {
        q.cursor = None;
        return Stopped(Stop::Done);
    }
    if input.powered {
        return Stopped(Stop::Redstone);
    }
    if q.side > input.max_side {
        return Stopped(Stop::TooLarge);
    }
    if !input.chest {
        return Stopped(Stop::NoChest);
    }
    if input.pending {
        return Stopped(Stop::ChestFull);
    }
    if q.wait > 0 {
        q.wait -= 1;
        return Action::Wait;
    }
    let here = q.area(pos);
    let fluid = match input.cell {
        Cell::Unloaded => return Stopped(Stop::Unloaded),
        Cell::Air | Cell::Unbreakable | Cell::Kept => {
            q.cursor = here.next(cursor, input.min_y);
            return Action::Skip;
        }
        Cell::Fluid if !input.upgrades.fluids => return Stopped(Stop::Fluid),
        Cell::Fluid => true,
        Cell::Solid => false,
    };
    if q.burn < BURN_PER_BLOCK {
        return match input.fuel {
            Some(_) => Action::Burn,
            None => Stopped(Stop::NoFuel),
        };
    }
    q.burn -= BURN_PER_BLOCK;
    q.wait = ticks_per_block(input.upgrades.speed) - 1;
    q.cursor = here.next(cursor, input.min_y);
    Action::Break {
        pos: here.pos(cursor),
        fluid,
    }
}

/// The number the screen reads: 0 is working, then `Stop` in order from 1.
pub fn status_code(action: &Action) -> i32 {
    match action {
        Action::Stopped(stop) => *stop as i32 + 1,
        _ => 0,
    }
}

// ---- chunks kept loaded, and the server's limit ----------------------------

/// A quarry: its dimension and position.
pub type Key = (String, [i32; 3]);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Hold {
    owner: u128,
    key: Key,
    chunks: Vec<[i32; 2]>,
}

/// The quarries keeping chunks loaded in a world.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loads {
    holds: Vec<Hold>,
}

/// What to do to the world's forced chunks, in the quarry's dimension.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Change {
    pub add: Vec<[i32; 2]>,
    pub remove: Vec<[i32; 2]>,
}

impl Loads {
    fn held_by_others(&self, key: &Key, chunk: [i32; 2]) -> bool {
        self.holds
            .iter()
            .any(|h| h.key != *key && h.key.0 == key.0 && h.chunks.contains(&chunk))
    }
    /// The quarry at `key` wants `chunks` loaded; none when its owner already
    /// has another quarry doing so.
    pub fn hold(&mut self, owner: u128, key: &Key, chunks: Vec<[i32; 2]>) -> Option<Change> {
        if self.holds.iter().any(|h| h.owner == owner && h.key != *key) {
            return None;
        }
        let before = self
            .holds
            .iter()
            .find(|h| h.key == *key)
            .map_or_else(Vec::new, |h| h.chunks.clone());
        let change = Change {
            add: chunks
                .iter()
                .filter(|c| !before.contains(c) && !self.held_by_others(key, **c))
                .copied()
                .collect(),
            remove: before
                .iter()
                .filter(|c| !chunks.contains(c) && !self.held_by_others(key, **c))
                .copied()
                .collect(),
        };
        self.holds.retain(|h| h.key != *key);
        self.holds.push(Hold {
            owner,
            key: key.clone(),
            chunks,
        });
        Some(change)
    }
    pub fn release(&mut self, key: &Key) -> Change {
        let Some(at) = self.holds.iter().position(|h| h.key == *key) else {
            return Change::default();
        };
        let hold = self.holds.remove(at);
        Change {
            add: Vec::new(),
            remove: hold
                .chunks
                .into_iter()
                .filter(|c| !self.held_by_others(key, *c))
                .collect(),
        }
    }
    pub fn holds(&self, key: &Key) -> bool {
        self.holds.iter().any(|h| h.key == *key)
    }
}

/// The server's setting, in the world's folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub max_side: i32,
}

impl Default for Config {
    fn default() -> Self {
        Config { max_side: side(3) }
    }
}

pub fn parse_max_side(arg: &str) -> Option<i32> {
    arg.parse().ok().filter(|n| [5, 9, 13, 17].contains(n))
}

#[cfg(test)]
mod upgrade_tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!([side(0), side(1), side(2), side(3)], [5, 9, 13, 17]);
    }
    #[test]
    fn speeds() {
        assert_eq!([0, 1, 2, 3].map(ticks_per_block), [40, 20, 10, 5]);
    }
    #[test]
    fn counts_slots() {
        let s = [Some(Upgrade::Speed), Some(Upgrade::Speed), Some(Upgrade::Fluids), None];
        let u = Upgrades::from_slots(&s);
        assert_eq!((u.area, u.speed, u.fluids, u.chunk), (0, 2, true, false));
    }
    #[test]
    fn limits() {
        let s = [Some(Upgrade::Area), Some(Upgrade::Area), Some(Upgrade::Area), None];
        assert!(!accepts(&s, 3, Upgrade::Area));
        assert!(accepts(&s, 3, Upgrade::Speed));
        let f = [Some(Upgrade::Fluids), None, None, None];
        assert!(!accepts(&f, 1, Upgrade::Fluids));
        assert!(accepts(&f, 0, Upgrade::Fluids)); // swapping in the same slot
        let c = [Some(Upgrade::Chunk), None, None, None];
        assert!(!accepts(&c, 2, Upgrade::Chunk));
    }
    #[test]
    fn item_names() {
        assert_eq!(upgrade_of("deepbore_quarry:speed_upgrade"), Some(Upgrade::Speed));
        assert_eq!(upgrade_of("deepbore_quarry:area_upgrade"), Some(Upgrade::Area));
        assert_eq!(upgrade_of("deepbore_quarry:fluids_upgrade"), Some(Upgrade::Fluids));
        assert_eq!(upgrade_of("deepbore_quarry:chunk_upgrade"), Some(Upgrade::Chunk));
        assert_eq!(upgrade_of("minecraft:coal"), None);
    }
}

#[cfg(test)]
mod area_tests {
    use super::*;

    #[test]
    fn in_front_and_centred() {
        let a = area([0, 64, 0], Facing::North, 5);
        assert_eq!((a.min, a.max, a.top), ([-2, -5], [2, -1], 64));
        let e = area([10, 70, 10], Facing::East, 9);
        assert_eq!((e.min, e.max, e.top), ([11, 6], [19, 14], 70));
    }
    #[test]
    fn walks_a_layer_then_goes_down() {
        let a = area([0, 64, 0], Facing::South, 5);
        let mut c = a.start();
        assert_eq!(c.y, 64);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..25 {
            assert!(seen.insert(a.pos(c)));
            c = a.next(c, -64).unwrap();
        }
        assert_eq!((c.y, c.index), (63, 0));
        assert!(seen.iter().all(|p| p[1] == 64 && a.contains(*p)));
    }
    #[test]
    fn ends_at_the_bottom() {
        let a = area([0, -64, 0], Facing::North, 5);
        let last = Cursor { y: -64, index: 24 };
        assert_eq!(a.next(last, -64), None);
    }
    #[test]
    fn below_the_world_is_done_at_once() {
        let a = area([0, -70, 0], Facing::North, 5);
        assert_eq!(a.next(a.start(), -64), None);
    }
    #[test]
    fn chunks_cover_area_quarry_and_chest() {
        let a = area([0, 64, 0], Facing::North, 17); // x -8..=8, z -17..=-1
        let c = a.chunks([0, 64, 0], Some([0, 64, 1]));
        assert_eq!(c, vec![[-1, -2], [-1, -1], [0, -2], [0, -1], [0, 0]]);
        let n = area([8, 64, 24], Facing::South, 5).chunks([8, 64, 24], None);
        assert_eq!(n, vec![[0, 1]]);
    }
}

#[cfg(test)]
mod step_tests {
    use super::*;

    const P: [i32; 3] = [0, 64, 0];

    fn none() -> Upgrades {
        Upgrades::default()
    }
    fn input() -> Input {
        Input {
            upgrades: none(),
            powered: false,
            max_side: 17,
            min_y: -64,
            chest: true,
            pending: false,
            fuel: None,
            cell: Cell::Solid,
        }
    }
    fn running() -> Quarry {
        let mut q = Quarry::new(7, Facing::North, P);
        q.on = true;
        q.burn = 1600;
        q
    }

    #[test]
    fn new_quarry_is_off() {
        let mut q = Quarry::new(7, Facing::North, P);
        assert!(matches!(step(&mut q, P, &input()), Action::Stopped(Stop::Off)));
    }
    #[test]
    fn placed_below_the_world_is_done() {
        let mut q = running();
        q.cursor = Some(Cursor { y: -70, index: 0 });
        assert!(matches!(step(&mut q, [0, -70, 0], &input()), Action::Stopped(Stop::Done)));
        assert_eq!(q.cursor, None);
    }
    #[test]
    fn one_coal_digs_eight_blocks() {
        let mut q = running();
        let mut broken = 0;
        for _ in 0..2000 {
            match step(&mut q, P, &input()) {
                Action::Break { .. } => broken += 1,
                Action::Stopped(Stop::NoFuel) => break,
                _ => {}
            }
        }
        assert_eq!(broken, 8);
    }
    #[test]
    fn pace_follows_speed() {
        let mut q = running();
        assert!(matches!(step(&mut q, P, &input()), Action::Break { .. }));
        for _ in 0..39 {
            assert!(matches!(step(&mut q, P, &input()), Action::Wait));
        }
        assert!(matches!(step(&mut q, P, &input()), Action::Break { .. }));
    }
    #[test]
    fn breaks_the_cell_under_the_cursor() {
        let mut q = running();
        let Action::Break { pos, fluid } = step(&mut q, P, &input()) else {
            panic!("no break");
        };
        assert_eq!((pos, fluid), ([-2, 64, -5], false));
        assert_eq!(q.target(P), Some([-1, 64, -5]));
    }
    #[test]
    fn skips_do_not_cost_fuel() {
        let mut q = running();
        for cell in [Cell::Air, Cell::Unbreakable, Cell::Kept] {
            assert!(matches!(step(&mut q, P, &Input { cell, ..input() }), Action::Skip));
        }
        assert_eq!(q.burn, 1600);
        assert_eq!(q.cursor.unwrap().index, 3);
    }
    #[test]
    fn fluid_stops_without_the_upgrade() {
        let mut q = running();
        let i = Input { cell: Cell::Fluid, ..input() };
        assert!(matches!(step(&mut q, P, &i), Action::Stopped(Stop::Fluid)));
        assert_eq!(q.cursor.unwrap().index, 0);
    }
    #[test]
    fn fluid_with_upgrade_never_stalls() {
        let mut q = running();
        let i = Input {
            cell: Cell::Fluid,
            upgrades: Upgrades { fluids: true, ..none() },
            ..input()
        };
        assert!(matches!(step(&mut q, P, &i), Action::Break { fluid: true, .. }));
        assert_eq!(q.cursor.unwrap().index, 1);
    }
    #[test]
    fn asks_to_burn_then_stops_dry() {
        let mut q = running();
        q.burn = 150;
        assert!(matches!(step(&mut q, P, &Input { fuel: Some(1600), ..input() }), Action::Burn));
        assert!(matches!(step(&mut q, P, &input()), Action::Stopped(Stop::NoFuel)));
    }
    #[test]
    fn stop_reasons_in_order() {
        let mut q = running();
        let chest = false;
        assert!(matches!(step(&mut q, P, &Input { powered: true, chest, ..input() }), Action::Stopped(Stop::Redstone)));
        assert!(matches!(step(&mut q, P, &Input { max_side: 4, chest, ..input() }), Action::Stopped(Stop::TooLarge)));
        assert!(matches!(step(&mut q, P, &Input { chest, pending: true, ..input() }), Action::Stopped(Stop::NoChest)));
        assert!(matches!(step(&mut q, P, &Input { pending: true, ..input() }), Action::Stopped(Stop::ChestFull)));
        assert_eq!(q.burn, 1600);
    }
    #[test]
    fn resize_restarts_from_the_top() {
        let mut q = running();
        q.cursor = Some(Cursor { y: 10, index: 7 });
        q.resize(P, 9);
        assert_eq!(q.cursor, Some(Cursor { y: 64, index: 0 }));
        q.cursor = Some(Cursor { y: 10, index: 7 });
        q.resize(P, 9); // the same size: nothing changes
        assert_eq!(q.cursor.unwrap().y, 10);
    }
    #[test]
    fn finishes_at_the_bottom() {
        let mut q = running();
        q.cursor = Some(Cursor { y: -64, index: 24 });
        assert!(matches!(step(&mut q, P, &input()), Action::Break { .. }));
        assert!(matches!(step(&mut q, P, &input()), Action::Stopped(Stop::Done)));
    }
    #[test]
    fn survives_a_save() {
        let q = Quarry::new(u128::MAX - 5, Facing::East, P);
        let back: Quarry = serde_json::from_str(&serde_json::to_string(&q).unwrap()).unwrap();
        assert_eq!((back.burn, back.cursor, back.owner, back.facing), (q.burn, q.cursor, q.owner, q.facing));
    }
    #[test]
    fn status_codes() {
        assert_eq!(status_code(&Action::Wait), 0);
        assert_eq!(status_code(&Action::Stopped(Stop::Off)), 1);
        assert_eq!(status_code(&Action::Stopped(Stop::Done)), 8);
    }
}

#[cfg(test)]
mod load_tests {
    use super::*;

    fn at(pos: [i32; 3]) -> Key {
        ("minecraft:overworld".to_string(), pos)
    }

    #[test]
    fn one_loading_quarry_per_owner() {
        let mut l = Loads::default();
        let (a, b) = (at([0, 64, 0]), at([100, 64, 0]));
        assert_eq!(l.hold(1, &a, vec![[0, 0]]).unwrap().add, vec![[0, 0]]);
        assert!(l.hold(1, &b, vec![[6, 0]]).is_none());
        assert!(l.hold(2, &b, vec![[6, 0]]).is_some());
        assert!(l.holds(&a) && l.holds(&b));
    }
    #[test]
    fn holding_again_changes_only_the_difference() {
        let mut l = Loads::default();
        let a = at([0, 64, 0]);
        l.hold(1, &a, vec![[0, 0], [0, 1]]);
        let c = l.hold(1, &a, vec![[0, 1], [0, 2]]).unwrap();
        assert_eq!((c.add, c.remove), (vec![[0, 2]], vec![[0, 0]]));
    }
    #[test]
    fn shared_chunk_stays_until_the_last_holder() {
        let mut l = Loads::default();
        let (a, b) = (at([0, 64, 0]), at([9, 64, 0]));
        l.hold(1, &a, vec![[0, 0]]);
        l.hold(2, &b, vec![[0, 0], [1, 0]]);
        assert!(l.release(&a).remove.is_empty());
        assert_eq!(l.release(&b).remove, vec![[0, 0], [1, 0]]);
        assert!(l.release(&b).remove.is_empty());
    }
    #[test]
    fn dimensions_do_not_share_chunks() {
        let mut l = Loads::default();
        let a = at([0, 64, 0]);
        let n = ("minecraft:the_nether".to_string(), [0, 64, 0]);
        l.hold(1, &a, vec![[0, 0]]);
        assert_eq!(l.hold(2, &n, vec![[0, 0]]).unwrap().add, vec![[0, 0]]);
        assert_eq!(l.release(&n).remove, vec![[0, 0]]);
    }
    #[test]
    fn max_side_values() {
        assert_eq!(
            ["5", "9", "13", "17", "7", "x"].map(parse_max_side),
            [Some(5), Some(9), Some(13), Some(17), None, None]
        );
        assert_eq!(Config::default().max_side, 17);
    }
}

#[cfg(test)]
mod later_tests {
    use super::*;

    #[test]
    fn an_unloaded_cell_is_not_skipped() {
        let mut q = Quarry::new(7, Facing::North, [0, 64, 0]);
        q.on = true;
        q.burn = 1600;
        let i = Input {
            upgrades: Upgrades::default(),
            powered: false,
            max_side: 17,
            min_y: -64,
            chest: true,
            pending: false,
            fuel: None,
            cell: Cell::Unloaded,
        };
        let a = step(&mut q, [0, 64, 0], &i);
        assert!(matches!(a, Action::Stopped(Stop::Unloaded)));
        assert_eq!(status_code(&a), 9);
        assert_eq!(q.cursor.unwrap().index, 0);
    }
    #[test]
    fn surplus_upgrades_add_nothing() {
        let area = [Some(Upgrade::Area); UPGRADE_SLOTS];
        assert_eq!(Upgrades::from_slots(&area).area, 3);
        let speed = [Some(Upgrade::Speed); UPGRADE_SLOTS];
        assert_eq!(Upgrades::from_slots(&speed).speed, 3);
    }
    #[test]
    fn digs_away_from_the_player() {
        assert_eq!(Facing::away_from("south"), Some(Facing::North));
        assert_eq!(Facing::away_from("north"), Some(Facing::South));
        assert_eq!(Facing::away_from("east"), Some(Facing::West));
        assert_eq!(Facing::away_from("west"), Some(Facing::East));
        assert_eq!(Facing::away_from("up"), None);
    }
    #[test]
    fn head_moves_towards_its_target() {
        assert_eq!(approach([0.0; 3], [10.0, 0.0, 0.0], 2.0), [2.0, 0.0, 0.0]);
        assert_eq!(approach([9.5, 0.0, 0.0], [10.0, 0.0, 0.0], 2.0), [10.0, 0.0, 0.0]);
        assert_eq!(approach([1.0; 3], [1.0; 3], 2.0), [1.0; 3]);
    }
    #[test]
    fn slots_are_named() {
        assert_eq!(FUEL_SLOT, 0);
        assert_eq!(FIRST_UPGRADE + UPGRADE_SLOTS, FIRST_BUFFER);
        assert_eq!(SLOTS, 14); // data/deepbore_quarry/horizon/menu/quarry.json says the same
    }
}
