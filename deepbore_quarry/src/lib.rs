//! Deepbore Quarry in the game: the edges between the game (`mod_api`) and the rules (`rules.rs`).
//!
//! A quarry's state lives in its block entity, so it moves with the chunk's save: `nbt["quarry"]` holds the
//! rules' `Quarry` as JSON, `items` its slots, `data` what its screen shows (`quarry.rs`). What belongs to
//! the world (the server's limit, the chunks held) lives in files in the world's folder (`world.rs`).

mod quarry;
mod rules;
mod world;

use mod_api::{
    BlockChangedEvent, BlockEntityTypeKey, BlockKey, BlocksEvent, BreakBlockEvent, Flow, Hooks, HostSide,
    InteractionResult, ItemKey, ItemsEvent, LevelHost, ModContainer, ParticleKey, PlayerLoggedInEvent,
    RegisterCommandsEvent, Registrar, RegistrationContext, RightClickBlockEvent, RunModCommandEvent, ServerHost,
    ServerTickEvent, TicketController, ValidateTicketsEvent,
};
use std::sync::OnceLock;

/// The registry keys the mod asks about, looked up once at registration.
pub(crate) struct Keys {
    pub block: BlockKey,
    pub tickets: TicketController,
    pub airs: [BlockKey; 3],
    pub cobblestone: BlockKey,
    pub pickaxe: ItemKey,
    pub bucket: ItemKey,
    pub lava_bucket: ItemKey,
    pub block_particle: ParticleKey,
    /// The upgrade items and their ids, which the rules read.
    pub upgrades: [(ItemKey, &'static str); 4],
}

static KEYS: OnceLock<Keys> = OnceLock::new();

pub(crate) fn keys() -> &'static Keys {
    KEYS.get().expect("the mod's keys are looked up at registration")
}

/// The upgrade an item is, if it is one.
pub(crate) fn upgrade_of(item: ItemKey) -> Option<rules::Upgrade> {
    let (_, id) = keys().upgrades.iter().find(|(key, _)| *key == item)?;
    rules::upgrade_of(id)
}

const UPGRADES: [&str; 4] = [
    "deepbore_quarry:area_upgrade",
    "deepbore_quarry:speed_upgrade",
    "deepbore_quarry:fluids_upgrade",
    "deepbore_quarry:chunk_upgrade",
];

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        // Full ids: a bare name would take the mod's id (deepbore_quarry_native) as its namespace, and a
        // saved quarry's block entity is `deepbore_quarry:quarry`.
        let block = r.block_key(rules::BLOCK);
        let kind: BlockEntityTypeKey = r.block_entity_type(rules::BLOCK).blocks(&[rules::BLOCK]).build();
        r.block_entity_part(&kind, quarry::Quarry).expect("the quarry's block entity type is the mod's own");
        // Hoppers put fuel in, from any side, and take nothing out.
        r.block_container(
            &block,
            ModContainer {
                slots: rules::SLOTS,
                faces: std::array::from_fn(|_| vec![rules::FUEL_SLOT]),
                inputs: vec![rules::FUEL_SLOT],
                outputs: Vec::new(),
                menu: Some(rules::MENU.to_owned()),
            },
        );
        r.menu_part(rules::MENU, quarry::QuarryMenu);
        let item = r.item_key(rules::BLOCK);
        r.item_part(item, quarry::QuarryItem);
        let tickets = r.register_ticket_controller(rules::BLOCK, true);
        let upgrades = UPGRADES.map(|id| (r.item_key(id), id));
        let _ = KEYS.set(Keys {
            block,
            tickets,
            airs: ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"].map(|id| r.block_key(id)),
            cobblestone: r.block_key("minecraft:cobblestone"),
            pickaxe: r.item_key("minecraft:diamond_pickaxe"),
            bucket: r.item_key("minecraft:bucket"),
            lava_bucket: r.item_key("minecraft:lava_bucket"),
            block_particle: r.particle_key("minecraft:block"),
            upgrades,
        });
    }

    fn blocks(&self, _: &RegistrationContext, event: &mut BlocksEvent<'_>) {
        world::blocks(event.blocks);
    }

    fn items(&self, _: &RegistrationContext, event: &mut ItemsEvent<'_>) {
        world::items(event.items);
    }

    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        world::commands(event.commands);
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        world::command(host, event)
    }

    /// Another player's quarry doesn't open, nor break.
    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        if quarry::refused(host, event.click.pos) {
            *event.cancellation_result = InteractionResult::Fail;
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        if quarry::refused(host, event.pos) { Flow::Cancel } else { Flow::Continue }
    }

    /// Any way a quarry goes: its chunks are let go.
    fn block_changed(&self, _: &dyn LevelHost, event: &mut BlockChangedEvent) {
        if event.old.block() == keys().block && event.new.block() != keys().block {
            world::quarry_gone(&event.dimension.key().to_string(), [event.pos.x, event.pos.y, event.pos.z]);
        }
    }

    fn validate_tickets(&self, host: &mut dyn ServerHost, event: &mut ValidateTicketsEvent<'_>) {
        if event.controller == keys().tickets {
            world::validate_tickets(host, event);
        }
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, mod_api::ServerTickPhase::ServerTickEnd { .. }) {
            world::server_tick(host);
        }
    }

    fn player_logged_in(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent<'_>) {
        world::logged_in(host, event.player);
    }
}
