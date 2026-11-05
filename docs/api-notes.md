# What the game's API turned out to be

Pinned on Nightly 67dad0d (API 359), horizon-mixins e8cd0aa.

- **Template:** `minecraft:furnace`. It opens a menu, and its `facing` points at the placer. The pit lies the opposite way.
- **Checks reject dead code:** `native.py check` builds with `-D warnings`, so anything in `runtime.rs` used only by tests needs `#[cfg(test)]`. `runtime.rs` can't have file attributes, `//!` comments included.
- **Slot rules run in the library crate:** `native_menu_slot_rules` has no game, and injected modules are `pub(crate)`, so it can't reach the glue or the game's fuel table. A click can put anything but an upgrade in the fuel slot. `native_menu_quick_move` has the game and checks fully.
- **Hidden slots:** slots past `NativeScreen.slots` are left out of the menu, and the engine saves them and drops them with the block. Drops waiting for room live there.
- **Sounds and particles** are on `GameLevel`: use `game.with_level(|_, level, _| ...)`.
- **Light:** `draw.light` is the block's own, dark inside an opaque block. Set it from the air above.
- **64 blocks:** the whole block entity drawing stops when the camera is more than 64 blocks from the quarry block. The bounds can't change that.
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
