# Sega's systems through PicoDrive

The core is a separate package, `libretro-picodrive`, built from this directory's PKGBUILD at a
pinned commit. **PicoDrive's licence is non-commercial** (its redistributions may not be sold or
used in a commercial product or activity): it is never part of any commercial distribution. The
licence is installed with the core. (Arch Linux ARM's own `libretro-picodrive` is labelled
BSD-3-Clause; the licence in PicoDrive's source, `COPYING`, is the one above.)

Installing it with `nibcase-picodrive` registers its options with DSlot for:

- **Mega-CD / Sega CD** discs in `Roms/SEGACD`: `.chd`, `.cue` with its tracks, `.iso`, or an
  `.m3u` listing a game's discs. The BIOS is the player's own, for the discs' region, in `BIOS/`:
  `bios_CD_U.bin` (US), `bios_CD_E.bin` (Europe) or `bios_CD_J.bin` (Japan). Without one,
  nibcase says which to add rather than start.
- **32X** carts in `Roms/32X` (`.32x`, or zipped). No BIOS needed. The 32X's two SH-2s run
  through the core's recompiler.
- **Pico** carts in `Roms/PICO` (Mega Drive dumps, `.md` or `.bin`, or zipped). No BIOS needed.
- **Mega Drive**, as its default: PicoDrive's description's priority is 90, blastem's 80, and a
  game's Game page offers blastem in its Emulator row.
- **Master System, Game Gear and SG-1000**, as a second choice: gearsystem (100) stays their
  default, and the Emulator row offers PicoDrive.
