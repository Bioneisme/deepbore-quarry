#!/usr/bin/env python3
"""Drives the game offscreen with horizon-ctl and checks the quarry.

    python tools/ingame_test.py GAME_FOLDER VERSION [SCENARIO ...]

GAME_FOLDER is a test instance's folder, VERSION the installed version it plays
(native-...). The world `dbq-test` is left for a look; delete the instance when done. Scenarios: screen dig chest kept fluid save
redstone limit hopper chunk draw recipes russian deep. Without any, all of them.
"""
import json
import os
import re
import subprocess
import sys
import time

GAME, VERSION = sys.argv[1:3]
ONLY = set(sys.argv[3:])
CTL = os.path.join(os.environ["APPDATA"], "horizon-launcher", "versions", VERSION, "horizon-ctl.exe")
os.environ["HORIZON_HOME"] = GAME
os.environ["MSYS_NO_PATHCONV"] = "1"
sys.stdout.reconfigure(encoding="utf-8")
SHOTS = os.path.join(GAME, "dbq-shots")
os.makedirs(SHOTS, exist_ok=True)
QUARRY, COAL = "deepbore_quarry:quarry", "minecraft:coal"
WORLD = "dbq-test"
failed = []


def ctl(*args, timeout=300):
    out = subprocess.run([CTL, *map(str, args)], capture_output=True, text=True, encoding="utf-8", timeout=timeout)
    try:
        return json.loads(out.stdout)
    except ValueError:
        return {"raw": (out.stdout + out.stderr)[:400]}


def chat(line):
    return ctl("command", line).get("chat", [])


def check(name, ok, detail=""):
    print(("ok  " if ok else "FAIL"), name, detail if not ok or detail else "", flush=True)
    if not ok:
        failed.append(name)


def block(x, y, z):
    return str(ctl("block", x, y, z).get("block", ctl("block", x, y, z)))


def shot(name):
    ctl("screenshot", os.path.join(SHOTS, f"{name}.png"))


def scene(ox, oz, filled=True):
    """A flat stage far from the last one: a quarry at (ox, 151, oz) digging north, a chest east of it."""
    chat(f"tp @s {ox + 0.5} 160 {oz + 8.5}")
    ctl("wait", 60)
    chat(f"fill {ox - 12} 150 {oz - 22} {ox + 12} 150 {oz + 8} minecraft:smooth_stone")
    chat(f"fill {ox - 12} 151 {oz - 22} {ox + 12} 155 {oz + 8} minecraft:air")
    if filled:
        chat(f"fill {ox - 2} 151 {oz - 5} {ox + 2} 151 {oz - 1} minecraft:smooth_stone")
    chat(f"tp @s {ox + 0.5} 151 {oz + 3.5} 180 0")
    ctl("wait", 10)
    ctl("give", QUARRY, 1)
    placed = ctl("place", ox, 151, oz, "--item", QUARRY)
    ctl("wait", 4)
    chat(f"setblock {ox + 1} 151 {oz} minecraft:chest")
    ctl("wait", 2)
    return placed


def opened(ox, oz):
    ctl("interact", ox, 151, oz)
    ctl("wait", 4)
    return ctl("menu")


def put(slot, item, count=1):
    """Moves `count` of an item from the inventory into container slot `slot` of the open menu."""
    ctl("give", item, count)
    ctl("wait", 2)
    at = next((s for s in ctl("screen").get("slots", []) if s.get("item") == item and s["slot"] < 36), None)
    if at is None:
        return False
    ctl("click-slot", at["slot"])
    ctl("click-slot", 300 + slot)
    ctl("wait", 2)
    return True


def container(slot):
    return next((s for s in ctl("screen").get("slots", []) if s["slot"] == 300 + slot), {})


def toggle(ox, oz):
    """Presses the screen's button; the menu must be open. The screen is centred: 176 by 166."""
    w, h = ctl("ui").get("gui_size", [640.0, 360.0])
    ctl("click-at", (w - 176) / 2 + 150, (h - 166) / 2 + 42)
    ctl("wait", 2)


def chest_counts(ox, oz, chest=None):
    cx, cy, cz = chest or (ox + 1, 151, oz)
    line = " ".join(chat(f"data get block {cx} {cy} {cz} Items"))
    counts = {}
    for m in re.finditer(r'count: ?(\d+)[^}]*?id: ?"([^"]+)"|id: ?"([^"]+)"[^}]*?count: ?(\d+)', line):
        if m[1]:
            counts[m[2]] = counts.get(m[2], 0) + int(m[1])
        else:
            counts[m[3]] = counts.get(m[3], 0) + int(m[4])
    return counts


def s_screen():
    placed = scene(0, 0)
    print(" placed:", str(placed)[:200])
    print(" state:", block(0, 151, 0))
    menu = opened(0, 0)
    check("right click opens the mod's screen", ctl("state").get("screen") in ("mod", "menu", "container"), str(ctl("state").get("screen")))
    shot("screen_off")
    check("the fuel slot takes coal", put(0, COAL, 8) and container(0).get("count") == 8, str(container(0)))
    shot("screen_coal")
    # An upgrade: one to a slot, never in the fuel slot.
    ctl("give", "deepbore_quarry:speed_upgrade", 2)
    ctl("wait", 2)
    at = next(s for s in ctl("screen")["slots"] if s.get("item") == "deepbore_quarry:speed_upgrade" and s["slot"] < 36)
    ctl("click-slot", at["slot"], "shift")
    ctl("wait", 2)
    check("a shift-click sends one upgrade to its slot", container(1).get("count") == 1, str(container(1)))
    check("and not to the fuel slot", container(0).get("item") == COAL, str(container(0)))
    ctl("click-slot", 301, "shift")
    ctl("wait", 2)
    check("it comes back with a shift-click", not container(1).get("item"), str(container(1)))
    ctl("close")
    ctl("wait", 2)


def s_dig():
    scene(100, 0)
    opened(100, 0)
    put(0, COAL, 1)
    toggle(100, 0)
    shot("screen_on")
    ctl("close")
    ctl("wait", 8 * 40 + 60)
    counts = chest_counts(100, 0)
    check("one coal digs eight blocks into the chest", counts.get("minecraft:smooth_stone") == 8, str(counts))
    shot("dug_eight")
    opened(100, 0)
    shot("screen_no_fuel")
    ctl("close")
    entities = ctl("entities")
    items = [e for e in entities.get("entities", []) if "item" in str(e.get("type", e.get("kind", "")))]
    check("nothing on the ground", not items, str(items)[:200])


def s_chest():
    scene(200, 0)
    opened(200, 0)
    put(0, COAL, 4)
    chat("setblock 201 151 0 minecraft:air")  # the chest goes before it starts
    toggle(200, 0)
    ctl("close")
    ctl("wait", 60)
    check("no chest: nothing is dug", "smooth_stone" in block(198, 151, -5), block(198, 151, -5))
    chat("setblock 201 151 0 minecraft:chest")
    ctl("wait", 100)
    check("a chest back: it digs on", chest_counts(200, 0).get("minecraft:smooth_stone", 0) >= 1, str(chest_counts(200, 0)))
    # A full chest: 27 stacks of cobblestone.
    items = ",".join(f'{{Slot:{i}b,id:"minecraft:cobblestone",count:64}}' for i in range(27))
    chat(f"data merge block 201 151 0 {{Items:[{items}]}}")
    ctl("wait", 120)
    stone = sum(1 for x in range(198, 203) for z in range(-5, 0) if "smooth_stone" in block(x, 151, z))
    ctl("wait", 80)
    stone2 = sum(1 for x in range(198, 203) for z in range(-5, 0) if "smooth_stone" in block(x, 151, z))
    check("a full chest stops it", stone == stone2, f"{stone} then {stone2}")
    chat("data merge block 201 151 0 {Items:[]}")
    ctl("wait", 120)
    check("room in the chest: the waiting drop arrives", chest_counts(200, 0).get("minecraft:smooth_stone", 0) >= 1, str(chest_counts(200, 0)))


def s_kept():
    scene(300, 0)
    chat("setblock 300 151 -5 minecraft:chest")
    chat('data merge block 300 151 -5 {Items:[{Slot:0b,id:"minecraft:diamond",count:3}]}')
    chat("setblock 301 151 -5 minecraft:bedrock")
    chat("setblock 299 151 -5 minecraft:obsidian")
    opened(300, 0)
    put(0, COAL, 4)
    toggle(300, 0)
    ctl("close")
    ctl("wait", 200)
    check("a chest in the area stays", "chest" in block(300, 151, -5), block(300, 151, -5))
    check("with its contents", chest_counts(300, 0, (300, 151, -5)).get("minecraft:diamond") == 3, str(chest_counts(300, 0, (300, 151, -5))))
    check("bedrock stays", "bedrock" in block(301, 151, -5), block(301, 151, -5))
    check("obsidian comes to the chest", chest_counts(300, 0).get("minecraft:obsidian") == 1, str(chest_counts(300, 0)))


def s_fluid():
    scene(400, 0)
    chat("setblock 398 151 -5 minecraft:water")
    opened(400, 0)
    put(0, COAL, 4)
    toggle(400, 0)
    ctl("close")
    ctl("wait", 60)
    check("water stops it without the upgrade", "smooth_stone" in block(399, 151, -5), block(399, 151, -5))
    opened(400, 0)
    shot("screen_fluid")
    put(1, "deepbore_quarry:fluids_upgrade", 1)
    ctl("close")
    ctl("wait", 100)
    check("with the upgrade the water is plugged", "cobblestone" in block(398, 151, -5), block(398, 151, -5))
    check("and it digs on", "smooth_stone" not in block(399, 151, -5), block(399, 151, -5))


def s_save():
    scene(500, 0)
    opened(500, 0)
    put(0, COAL, 4)
    toggle(500, 0)
    ctl("close")
    ctl("wait", 120)
    before = chest_counts(500, 0).get("minecraft:smooth_stone", 0)
    ctl("quit")
    time.sleep(3)
    launched()
    chat("tp @s 500.5 151 3.5")
    ctl("wait", 120)
    after = chest_counts(500, 0).get("minecraft:smooth_stone", 0)
    print(" before", before, "after", after)
    check("digs on after the world is reopened", after > before, f"{before} then {after}")
    opened(500, 0)
    shot("screen_reopened")
    ctl("close")


def s_redstone():
    scene(600, 0)
    chat("setblock 600 151 1 minecraft:redstone_block")
    opened(600, 0)
    put(0, COAL, 4)
    toggle(600, 0)
    ctl("close")
    ctl("wait", 60)
    check("redstone stops it", "smooth_stone" in block(598, 151, -5), block(598, 151, -5))
    chat("setblock 600 151 1 minecraft:air")
    ctl("wait", 60)
    check("no signal: it works", "smooth_stone" not in block(598, 151, -5), block(598, 151, -5))


def s_limit():
    scene(700, 0)
    opened(700, 0)
    put(0, COAL, 4)
    put(1, "deepbore_quarry:area_upgrade", 1)
    ctl("close")
    print(" maxsize:", chat("deepbore maxsize 5"))
    opened(700, 0)
    toggle(700, 0)
    ctl("wait", 20)
    shot("screen_limit")
    ctl("close")
    ctl("wait", 60)
    check("area over the limit: nothing dug", "smooth_stone" in block(698, 151, -5), block(698, 151, -5))
    print(" maxsize:", chat("deepbore maxsize 17"))
    ctl("wait", 80)
    check("the limit lifted: it digs", chest_counts(700, 0).get("minecraft:smooth_stone", 0) >= 1, str(chest_counts(700, 0)))


def s_hopper():
    scene(800, 0)
    chat("setblock 800 152 0 minecraft:hopper[facing=down]")
    chat('data merge block 800 152 0 {Items:[{Slot:0b,id:"minecraft:coal",count:2}]}')
    chat("setblock 800 150 0 minecraft:hopper")
    ctl("wait", 60)
    opened(800, 0)
    fuel = container(0)
    check("a hopper feeds the fuel slot", fuel.get("item") == COAL, str(fuel))
    ctl("close")
    check("a hopper below takes nothing", chest_counts(800, 0, (800, 150, 0)) == {}, str(chest_counts(800, 0, (800, 150, 0))))


def s_chunk():
    scene(900, 0)
    opened(900, 0)
    put(0, COAL, 16)
    put(1, "deepbore_quarry:chunk_upgrade", 1)
    toggle(900, 0)
    ctl("close")
    ctl("wait", 40)
    print(" forceload:", chat("forceload query"))
    a = chest_counts(900, 0).get("minecraft:smooth_stone", 0)
    chat("tp @s 5000 200 5000")
    ctl("wait", 200)
    chat("tp @s 900.5 151 3.5")
    ctl("wait", 40)
    b = chest_counts(900, 0).get("minecraft:smooth_stone", 0)
    check("with the upgrade it works while the player is away", b >= a + 3, f"{a} then {b}")
    # In survival, with a pickaxe: creative mode drops no block.
    chat("gamemode survival")
    ctl("give", "minecraft:iron_pickaxe", 1)
    ctl("select", "minecraft:iron_pickaxe")
    print(" break:", str(ctl("break", 900, 151, 0))[:160])
    ctl("wait", 20)
    chat("gamemode creative")
    line = " ".join(chat("forceload query"))
    print(" forceload after:", line)
    check("broken: its chunks are let go", "No chunks" in line or "not" in line.lower() or "56" not in line, line)
    entities = json.dumps(ctl("entities"))
    check("broken: it drops itself", QUARRY in entities, entities[:200])


def s_draw():
    scene(1000, 0)
    opened(1000, 0)
    put(0, COAL, 8)
    put(1, "deepbore_quarry:area_upgrade", 1)
    put(2, "deepbore_quarry:speed_upgrade", 1)
    toggle(1000, 0)
    ctl("close")
    chat("tp @s 1006.5 158 8.5 150 25")
    ctl("wait", 40)
    shot("gantry_a")
    ctl("wait", 6)
    shot("gantry_b")
    ctl("wait", 6)
    shot("gantry_c")
    print(" state:", json.dumps(ctl("status"))[:300])


def s_recipes():
    for name in ["quarry", "area_upgrade", "speed_upgrade", "fluids_upgrade", "chunk_upgrade"]:
        line = " ".join(chat(f"recipe give @s deepbore_quarry:{name}"))
        check(f"the recipe of {name}", "Unlocked" in line or "unlocked" in line, line)


def s_russian():
    scene(1100, 0)
    was = ctl("options-get", "lang").get("value", "en_us")
    print(" lang was:", was)
    ctl("options-set", "lang", "ru_ru")
    ctl("wait", 40)
    opened(1100, 0)
    shot("screen_ru")
    ctl("close")
    ctl("options-set", "lang", was)
    ctl("wait", 20)


def s_deep():
    """Real ground: a pit with everything the world has in it, as fast as the quarry goes."""
    chat("tp @s 2000 200 2000")
    ctl("wait", 120)
    y = None
    for yy in range(140, 40, -1):
        if "air" not in block(2000, yy, 2000) and "leaves" not in block(2000, yy, 2000):
            y = yy
            break
    print(" ground at", y)
    level = y + 1
    chat(f"tp @s 2000.5 {level} 2003.5 180 0")
    ctl("wait", 20)
    ctl("give", QUARRY, 1)
    print(" place:", str(ctl("place", 2000, level, 2000, "--item", QUARRY))[:150])
    ctl("wait", 2)
    chat(f"setblock 2001 {level} 2000 minecraft:chest")
    ctl("interact", 2000, level, 2000)
    ctl("wait", 4)
    put(0, "minecraft:coal_block", 16)
    put(1, "deepbore_quarry:speed_upgrade", 1)
    put(2, "deepbore_quarry:speed_upgrade", 1)
    put(3, "deepbore_quarry:speed_upgrade", 1)
    put(4, "deepbore_quarry:fluids_upgrade", 1)
    toggle(2000, 2000)
    shot("deep_screen")
    ctl("close")
    chat(f"tp @s 2006.5 {level + 8} 2006.5 150 30")
    ctl("wait", 600)
    shot("deep_pit")
    counts = chest_counts(2000, 2000, (2001, level, 2000))
    print(" chest:", counts)
    check("real ground comes to the chest", sum(counts.values()) >= 60, str(counts))
    items = [e for e in ctl("entities").get("entities", []) if e.get("kind") == "item"]
    check("nothing lands on the ground", not items, str(items)[:300])


SCENARIOS = {
    "screen": s_screen, "dig": s_dig, "chest": s_chest, "kept": s_kept, "fluid": s_fluid, "save": s_save,
    "redstone": s_redstone, "limit": s_limit, "hopper": s_hopper, "chunk": s_chunk, "draw": s_draw,
    "recipes": s_recipes, "russian": s_russian, "deep": s_deep,
}


def launched():
    r = ctl("launch", "--world", WORLD, "--seed", "12345", "--creative", "--wait", "--offscreen", timeout=280)
    print(" launch:", str(r)[:100], flush=True)


def main():
    launched()
    print(" screen:", ctl("state").get("screen"))
    for name, run in SCENARIOS.items():
        if not ONLY or name in ONLY:
            print("=====", name, flush=True)
            try:
                run()
            except Exception as e:  # keep going: the other scenarios still tell something
                check(f"{name} ran", False, repr(e)[:300])
    ctl("quit")
    print(f"== failed: {failed}" if failed else "== all passed", flush=True)


if __name__ == "__main__":
    main()
