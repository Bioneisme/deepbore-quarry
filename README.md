# Deepbore Quarry

A native mod for [Horizon](https://github.com/horizon-rust/horizon). It adds a machine that digs a square pit down to bedrock on furnace fuel and fills a chest beside it, under a moving gantry with a drill.

## How to use it

1. Craft the quarry: three diamonds, two pistons, an iron pickaxe, two iron blocks and redstone.
2. Place it. The pit lies beyond it, the way you are looking. It digs from the quarry's own level down.
3. Put a chest against any other side of it.
4. Right-click it, put in fuel, and press Start.

One coal digs eight blocks. Without upgrades the pit is 5×5 and a block takes two seconds. The quarry stops, and says why on its screen, when it runs out of fuel or the chest is full. A redstone signal stops it too.

It leaves bedrock and any block with contents, such as chests and furnaces. Breaking the quarry gives back the quarry, its fuel and its upgrades. The pit stays.

## Upgrades

A quarry has four upgrade slots, so you choose what it is good at.

| Upgrade | Most | What it does |
|---|---|---|
| Area | 3 | 5×5, then 9×9, 13×13 and 17×17 |
| Speed | 3 | a block in 2 s, then 1 s, 0.5 s and 0.25 s |
| Fluids | 1 | plugs water and lava with cobblestone and digs on, instead of stopping |
| Chunk | 1 | keeps working when you are away, one quarry per player |

## On a server

Only the player who placed a quarry, or an operator, can open or break it. Operators can limit the area with `/deepbore maxsize <5|9|13|17>`.

## Installing

Install both entries from the catalog: `deepbore_quarry` (assets) and `deepbore_quarry_native` (code), then build in the launcher. If one is missing, the game says so in chat. Horizon clients only.

## License

MIT
