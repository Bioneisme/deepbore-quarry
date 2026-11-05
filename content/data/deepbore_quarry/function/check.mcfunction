# Once for each player: tell them when the native half of the mod isn't there.
# Only that half adds to the tag, and every block out of the world is in it then.
execute as @a[tag=!deepbore_quarry.told] at @s positioned ~ 2000 ~ unless block ~ ~ ~ #deepbore_quarry:native_present run tellraw @s {"translate":"deepbore_quarry.missing.native","color":"red"}
tag @a[tag=!deepbore_quarry.told] add deepbore_quarry.told
