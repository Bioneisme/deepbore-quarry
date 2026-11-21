//! What belongs to the world: its files (the server's limit, the chunks held), the chunk tickets, the
//! declarations and the command.

use crate::{keys, rules};
use mod_api::{
    Block, BlockProperties, Command, CommandArgumentValue, Flow, Item, Rarity, RunModCommandEvent,
    ServerHost, Text, TicketOwner, ValidateTicketsEvent, text::TextStyle,
};
use rules::store;
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

pub(crate) struct WorldState {
    dir: PathBuf,
    pub config: rules::Config,
    /// The quarries holding chunk tickets (holds.json).
    pub loads: rules::Loads,
    /// Not yet saved. Cleared only by a save that succeeded, so a failed one is tried again.
    pub loads_changed: bool,
    /// A damaged holds.json couldn't be moved aside: it isn't written over in this session.
    loads_blocked: bool,
    /// The last save of holds.json failed (logged once, not every tick).
    loads_failing: bool,
    /// Said in chat to whoever joins: a world file was damaged.
    notices: Vec<String>,
    /// Ticket changes for the end of the server tick, outside any block entity's tick.
    pub tickets: Vec<Ticket>,
    /// 0.1.x's own `/forceload`s, removed once at the first tick (from an old loads.json).
    unforce: Vec<(String, [i32; 2])>,
    /// The first tick of this world has run: the saved holds are tickets again.
    woken: bool,
    /// Ticks spent waiting for a level with saved holds to open.
    wake_tries: u32,
}

/// One chunk a quarry holds or lets go of, under its own ticket.
pub(crate) struct Ticket {
    dimension: String,
    quarry: [i32; 3],
    chunk: [i32; 2],
    hold: bool,
}

static WORLD: Mutex<Option<WorldState>> = Mutex::new(None);

fn lock() -> MutexGuard<'static, Option<WorldState>> {
    WORLD.lock().unwrap_or_else(|e| e.into_inner())
}

/// A world file, or an empty one when there is none. A damaged file is moved aside
/// (`<name>.corrupt-<seconds>`), logged as an error and said in chat; the answer's `false` means it
/// couldn't be moved, so it must not be written over.
fn read_json<T: serde::de::DeserializeOwned + Default>(path: &Path, notices: &mut Vec<String>) -> (T, bool) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    match store::read_json(path, now) {
        Ok(value) => (value, true),
        Err(store::Damaged { error, kept }) => {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            let (line, writable) = match &kept {
                Some(kept) => {
                    let kept = kept.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                    (format!("{name} could not be read ({error}); it was kept as {kept} and a new one is started."), true)
                }
                None => (
                    format!(
                        "{name} could not be read ({error}) nor moved aside; it is left as it is and not saved over until the world is reopened."
                    ),
                    false,
                ),
            };
            log::error!("[deepbore_quarry] {}: {line}", path.display());
            notices.push(format!("Deepbore Quarry: {line}"));
            (T::default(), writable)
        }
    }
}

/// Writes a world file and logs a failure; the caller keeps what it couldn't save.
fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    store::write_json(path, value).inspect_err(|e| {
        log::error!("[deepbore_quarry] {} was not saved: {e}", path.display());
    })
}

/// The state of the open world, read on first use and again when another world opens.
pub(crate) fn world(host: &dyn ServerHost) -> Option<MutexGuard<'static, Option<WorldState>>> {
    let dir = host.world_dir()?.join(rules::DIR);
    let mut state = lock();
    if state.as_ref().is_none_or(|s| s.dir != dir) {
        let mut notices = Vec::new();
        let (config, _) = read_json(&dir.join("config.json"), &mut notices);
        let (mut loads, loads_writable) = read_json::<rules::Loads>(&dir.join(rules::HOLDS_FILE), &mut notices);
        let (unforce, migrated) = migrate_old_loads(&dir, &mut loads, &mut notices);
        *state = Some(WorldState {
            config,
            loads,
            loads_changed: migrated,
            loads_blocked: !loads_writable,
            loads_failing: false,
            notices,
            tickets: Vec::new(),
            unforce,
            woken: false,
            wake_tries: 0,
            dir,
        });
    }
    Some(state)
}

/// A loads.json from 0.1.x, which forced chunks with `/forceload`: its holds become tickets, its own
/// forced chunks are returned to be removed once, and the file is renamed loads.json.migrated. True when
/// holds.json is to be saved.
fn migrate_old_loads(
    dir: &Path,
    loads: &mut rules::Loads,
    notices: &mut Vec<String>,
) -> (Vec<(String, [i32; 2])>, bool) {
    let path = dir.join(rules::OLD_LOADS_FILE);
    if !path.exists() {
        return (Vec::new(), false);
    }
    let (old, _) = read_json::<rules::OldLoads>(&path, notices);
    let (holds, unforce) = rules::migrate(old);
    let carry = *loads == rules::Loads::default() && holds != rules::Loads::default();
    if carry {
        *loads = holds;
    }
    let done = dir.join(format!("{}.migrated", rules::OLD_LOADS_FILE));
    match std::fs::rename(&path, &done) {
        Ok(()) => log::info!(
            "[deepbore_quarry] {} moved to chunk tickets ({} forced chunk(s) to release); kept as {}",
            path.display(),
            unforce.len(),
            done.display()
        ),
        Err(e) => log::error!(
            "[deepbore_quarry] {} could not be renamed after moving to chunk tickets: {e}",
            path.display()
        ),
    }
    (unforce, carry)
}

/// Queues a quarry's ticket change for the end of the server tick.
pub(crate) fn ticket(tickets: &mut Vec<Ticket>, key: &rules::Key, change: rules::Change) {
    for (hold, chunks) in [(true, change.add), (false, change.remove)] {
        for chunk in chunks {
            tickets.push(Ticket {
                dimension: key.0.clone(),
                quarry: key.1,
                chunk,
                hold,
            });
        }
    }
}

/// Takes or lets go of one chunk under a quarry's ticket. False where the level isn't open.
fn force(host: &mut dyn ServerHost, dimension: &str, quarry: [i32; 3], chunk: [i32; 2], hold: bool) -> bool {
    let Some(level) = host.level(dimension) else {
        return false;
    };
    let Some(chunk_ref) = level.chunk_ref(host, chunk) else {
        return false;
    };
    chunk_ref.set_mod_forced(host, keys().tickets, TicketOwner::Block(quarry), hold, false);
    true
}

/// A quarry is gone, however it went: its chunks are let go.
pub(crate) fn quarry_gone(dimension: &str, pos: [i32; 3]) {
    let mut state = lock();
    let Some(st) = state.as_mut() else {
        return;
    };
    let key = (dimension.to_owned(), pos);
    if st.loads.holds(&key) {
        let change = st.loads.release(&key);
        ticket(&mut st.tickets, &key, change);
        st.loads_changed = true;
    }
}

/// The world's saved tickets, as it loads: a quarry's ticket stays only while holds.json knows it.
pub(crate) fn validate_tickets(host: &mut dyn ServerHost, event: &mut ValidateTicketsEvent<'_>) {
    let Some(guard) = world(host) else {
        return;
    };
    let st = guard.as_ref().expect("opened");
    let dimension = event.dimension.to_owned();
    for (pos, _) in event.validation.block_tickets() {
        if !st.loads.holds(&(dimension.clone(), pos)) {
            event.validation.remove_block_tickets(pos);
        }
    }
}

/// The end of each server tick: what had to wait for it.
pub(crate) fn server_tick(host: &mut dyn ServerHost) {
    let (tickets, unforce, wake) = {
        let Some(mut guard) = world(host) else {
            return;
        };
        let st = guard.as_mut().expect("opened");
        // The game keeps tickets with the world now, but those of 0.2 and older are dropped as it
        // loads: at the first tick every saved hold is taken again (a chunk held already stays held).
        let wake = if st.woken { Vec::new() } else { st.loads.tickets() };
        st.woken = true;
        (std::mem::take(&mut st.tickets), std::mem::take(&mut st.unforce), wake)
    };
    // Horizon opens the Nether and the End when someone goes there, and a level's saved tickets don't
    // open it as the world opens (vanilla's levels all open with the server). A command run in the
    // level opens it; it then loads its saved tickets and stays open while they hold its chunks.
    let mut opening: Vec<String> = Vec::new();
    for (dimension, quarry, chunk) in wake {
        if host.level(&dimension).is_none() {
            if !opening.contains(&dimension) {
                host.commands().run_command(&format!("execute in {dimension} run forceload query"));
                opening.push(dimension);
            }
            continue;
        }
        force(host, &dimension, quarry, chunk, true);
    }
    if !opening.is_empty()
        && let Some(st) = lock().as_mut()
    {
        // Again at the next tick, for a while: the level opens after the command's tick.
        st.wake_tries += 1;
        if st.wake_tries < 100 {
            st.woken = false;
        } else {
            log::warn!("[deepbore_quarry] {} didn't open: their quarries wait for someone to go there", opening.join(", "));
        }
    }
    // A world from 0.1.x: its own /forceloads go, once; the quarries hold tickets now.
    for (dimension, [x, z]) in unforce {
        host.commands().run_command(&format!(
            "execute in {dimension} run forceload remove {} {}",
            x * 16,
            z * 16
        ));
    }
    for t in tickets {
        force(host, &t.dimension, t.quarry, t.chunk, t.hold);
    }
    if let Some(st) = lock().as_mut()
        && !st.loads_blocked
    {
        let path = st.dir.join(rules::HOLDS_FILE);
        // A failure leaves loads_changed set: the next tick tries again. It is logged once, when
        // saving starts failing, and again when it works.
        match store::save_if_changed(&mut st.loads_changed, || store::write_json(&path, &st.loads)) {
            Err(e) if !st.loads_failing => {
                log::error!("[deepbore_quarry] {} was not saved, trying again each tick: {e}", path.display());
                st.loads_failing = true;
            }
            Ok(()) if st.loads_failing => {
                log::info!("[deepbore_quarry] {} is saved again", path.display());
                st.loads_failing = false;
            }
            _ => {}
        }
    }
}

/// The server's limit on a quarry's side.
pub(crate) fn max_side(host: &dyn ServerHost) -> i32 {
    world(host).map_or(rules::side(3), |g| g.as_ref().expect("opened").config.max_side)
}

pub(crate) fn red(line: Text) -> Text {
    let mut style = TextStyle::default();
    style.color = Some("red".to_owned());
    line.with_style(style)
}

/// A player joins: a damaged world file, moved aside when the world opened, is said to everyone.
pub(crate) fn logged_in(host: &mut dyn ServerHost, player: u128) {
    let notices = world(host).and_then(|guard| guard.as_ref().map(|st| st.notices.clone()));
    let Some(player) = host.player(player) else {
        return;
    };
    for line in notices.unwrap_or_default() {
        let _ = player.tell(host, red(Text::literal(line)));
    }
}

// ---- declarations --------------------------------------------------------------

/// The quarry. A furnace is its template: it opens a menu when used, and its `facing` is the side
/// towards the player who placed it.
pub(crate) fn blocks(blocks: &mut Vec<Block>) {
    let mut block = Block::new(rules::BLOCK, "minecraft:furnace").with_item(true);
    block.properties = BlockProperties {
        hardness: Some(3.5),
        explosion_resistance: Some(6.0),
        requires_tool: Some(true),
        sounds: Some("minecraft:netherite_block".into()),
        ..Default::default()
    };
    blocks.push(block);
}

pub(crate) fn items(items: &mut Vec<Item>) {
    for id in crate::UPGRADES {
        let mut item = Item::new(id, "minecraft:paper");
        item.max_stack_size = Some(16);
        item.rarity = Some(Rarity::Uncommon);
        items.push(item);
    }
}

pub(crate) fn commands(commands: &mut Vec<Command>) {
    commands.push(Command::op("deepbore maxsize", 2));
    commands.push(Command::op("deepbore maxsize <size:text>", 2));
}

fn size_line(side: i32) -> Text {
    let n = Text::literal(side.to_string());
    Text::translate("deepbore_quarry.maxsize.set", vec![n.clone(), n])
}

pub(crate) fn command(host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
    if event.command.literals.first().map(String::as_str) != Some("deepbore") {
        return Flow::Continue;
    }
    let size = match event.command.argument("size") {
        Some(CommandArgumentValue::String(size)) => Some(size),
        _ => None,
    };
    let Some(mut guard) = world(host) else {
        return Flow::Cancel;
    };
    let st = guard.as_mut().expect("opened");
    let reply = match (event.command.literals.get(1).map(String::as_str), size) {
        (Some("maxsize"), Some(arg)) => match rules::parse_max_side(&arg) {
            Some(side) => {
                st.config.max_side = side;
                match write_json(&st.dir.join("config.json"), &st.config) {
                    Ok(()) => size_line(side),
                    Err(e) => size_line(side).append(Text::literal(format!(" (not saved: {e})"))),
                }
            }
            None => Text::translate("deepbore_quarry.maxsize.usage", Vec::new()),
        },
        (Some("maxsize"), None) => size_line(st.config.max_side),
        _ => Text::translate("deepbore_quarry.maxsize.usage", Vec::new()),
    };
    drop(guard);
    let _ = host.commands().feedback(reply);
    Flow::Cancel
}
