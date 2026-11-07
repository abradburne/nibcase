# Arcade through FinalBurn Neo

The core is a separate package, `libretro-fbneo`, built from this directory's PKGBUILD at a pinned
commit. **FinalBurn Neo's licence is non-commercial** (it may not be sold or otherwise used for
profit, and projects using it may not ask for donations): it is never part of any commercial
distribution. The licence is installed with the core.

Installing it with `nibcase-fbneo` registers its options with DSlot for arcade games in
`Roms/ARCADE`: each game a set as FinalBurn Neo names it, `.zip` or `.7z` (`mslug.zip`,
`sf2.zip`), read whole by the core. Sets must match the core's version (the DAT in its source,
`dats/`); a set that doesn't is refused with the files it lacks.

The package also installs `/usr/share/libretro-fbneo/arcade.tsv`, every arcade set's title, maker
and year from that DAT and its board's family from the core's `gamelist.txt` (Neo Geo, CPS-1,
CPS-2, CPS-3, PGM, Cave, Toaplan, Konami, Sega, Taito, ...), which DSlot names the games on the
shelf by and splits its Arcade page into sections by.

**BIOS sets** are the player's own: `neogeo.zip` for Neo Geo games, `pgm.zip`, `skns.zip` and the
others the core's documentation lists. Put them in `BIOS/` (or beside the games). They are not
games and are not shown.

Games are shown at their cabinet monitor's shape (4:3, or 3:4 for an upright monitor), by whole
steps of height.
