# PlayStation through PCSX ReARMed

The core is a separate package, `libretro-pcsx-rearmed-nibos`, built from this directory's PKGBUILD
at a pinned commit (GPL-2.0) with its ARM64 dynamic recompiler; it replaces Arch Linux ARM's 2016
build. Installing it with `nibcase-pcsx-rearmed` registers its options with DSlot for PlayStation
discs in `Roms/PS`: `.chd`, `.cue` (with its `.bin` tracks), `.pbp`, or an `.m3u` listing a game's
discs, one file name a line.

- **BIOS:** optional. With none, the core's own (HLE) plays most games; nibcase says so in the
  menu. For the best results put one of the files libretro's PCSX ReARMed page lists in `BIOS/`
  at the root of either card: `scph5501.bin` (recommended), `scph1001.bin`, `scph7001.bin`,
  `scph101.bin` or `PSXONPSP660.bin`. See nibcase's docs/systems.md for their checksums.
- **Memory cards:** card 1 is the game's own, kept in its saves folder as `<game>.srm`.
- **Discs:** a game listed in an `.m3u` changes disc from the Play page of the menu (Disc).
- **Controller:** Digital pad by default; DualShock (the stick as the left analog stick) per game
  or for every PlayStation game, in the menu.
