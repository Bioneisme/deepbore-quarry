# Tell each player, once, when the native half of the mod isn't there: that half
# tags every player as they join, and the tag is missing without it.
execute as @a[tag=!deepbore_quarry.native,tag=!deepbore_quarry.told] run tellraw @s {"translate":"deepbore_quarry.missing.native","color":"red","extra":[{"text":" "},{"text":"https://playhorizon.dev/mods/deepbore_quarry_native","color":"aqua","underlined":true,"click_event":{"action":"open_url","url":"https://playhorizon.dev/mods/deepbore_quarry_native"}}]}
tag @a[tag=!deepbore_quarry.native,tag=!deepbore_quarry.told] add deepbore_quarry.told
