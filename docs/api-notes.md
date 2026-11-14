# What the game's API turned out to be

Pinned on Nightly 9f42064 (API 415), horizon-mixins 5a64af9.

- **Template:** `minecraft:furnace`. It opens a menu, and its `facing` points at the placer. The pit lies the opposite way.
- **Checks reject dead code:** `native.py check` builds with `-D warnings`, so anything in `runtime.rs` used only by tests needs `#[cfg(test)]`. `runtime.rs` can't have file attributes, `//!` comments included.
- **Slot rules run in the library crate:** `native_menu_slot_rules` has no game, and injected modules are `pub(crate)`, so it can't reach the glue or the game's fuel table. A click can put anything but an upgrade in the fuel slot. `native_menu_quick_move` has the game and checks fully.
- **Hidden slots:** slots past `NativeScreen.slots` are left out of the menu, and the engine saves them and drops them with the block. Drops waiting for room live there.
- **Sounds and particles** are on `GameLevel`: use `game.with_level(|_, level, _| ...)`.
- **Light:** `draw.light` is the block's own, dark inside an opaque block. Set it from the air above.
- **64 blocks:** the whole block entity drawing stops when the camera is more than 64 blocks from the quarry block. The bounds can't change that.
- **Embedded content:** the content mod rides in the bundle's `data/` (`native_data!`) and an injection at the
  head of `game::mods::content_root` hands it to the game as a dev mod. On 9f42064 (Horizon #11083) that function
  takes `(root, enabled, disabled, dev)`, so the injection binds `disabled` too, and `hooks` is 415 so the launcher
  never builds it on an older Horizon. #11083 also lets a native `.hmod` carry the content beside the code
  (`native-content/<id>/`), but on 2026-11-14 launcher 0.2.61 didn't put that content into the build (raw keys in
  the game, found on Wayfarer Stones), so the mod stays an `.hnmod`.
- **Chunk tickets (API 412/413, Horizon #12046):** `native_force_chunk_in(owner, dimension, chunk, hold)` holds a
  chunk loaded and ticking for an owner string; `/forceload` neither lists nor releases it. Tickets aren't saved, so
  `holds.json` keeps what each quarry holds and the first server tick takes it again (naming the level opens one
  nobody is in). Each quarry has its own owner (`deepbore_quarry:quarry/<dimension>/<x>,<y>,<z>`), so two quarries
  on one chunk don't need the old bookkeeping of who forced what.
- **`native_player_break_block` (#12259)** breaks as an online player, wears their held item and counts their
  exhaustion. A quarry digs while its owner is away or offline, so it keeps its own drops.
- **Internals that may change with a revision:** `Providers::burn_time`, `Items::block_drops`, `Items::block_loot_table`, `StateFacts`. All in one section of `glue.rs`.

## Building and checking

- The rules: `cargo test`. On Windows without MSVC, `cargo +stable-x86_64-pc-windows-gnu test`.
- The game with the mod applied, and the release files: `tools/build.sh check` and `tools/build.sh package`, on Linux or WSL.
- In the game, on a test instance: `tools/ingame_test.py` and `tools/halves_test.py`. They drive it offscreen with `horizon-ctl`, and each says how to run it.
- `tools/make_textures.py` redraws the textures and the icon.

## Not tested

- A player who isn't the owner: opening and breaking are refused in the code, but one client can't try it.
- Two owners' quarries sharing a chunk in the game. The rules' tests cover it.
- A dedicated server.
