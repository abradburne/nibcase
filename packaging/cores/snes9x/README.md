# Snes9x for nibcase

This package selects Arch Linux ARM's `libretro-snes9x` as a separate dependency. It does not
contain or link the core into nibcase. Its upstream noncommercial licence remains the core's
licence: see the installed `libretro-snes9x` licence before redistributing that core.

Installing or upgrading the core automatically queries its options and writes
`/usr/share/dslot/emulators/nibcase-snes9x.toml`. DSlot discovers this without editing a config.
Removing the core removes its generated integration; per-game settings and saves are kept.
