# Systems, cores and BIOS

What nibcase plays, through which core, under which licence, and what the player supplies. The
table in `crates/nibcase-libretro/src/systems.rs` is the source of truth; this page explains it.
To add a system or a core, see [adding a core](adding-a-core.md).

## The systems

Folders are the stock OS's names, under `Roms/` on either card; saves go in
`Saves/<folder>/<game>/` on the game's card. Cartridge systems also take a `.zip` holding one
game; disc systems never do.

| System | Folder | Files | Core | Core licence | Core package |
|---|---|---|---|---|---|
| SNES | `SFC` | `.sfc` `.smc` | Snes9x | **Non-commercial** (Snes9x licence) | `libretro-snes9x` (Arch Linux ARM) |
| NES | `FC` | `.nes` `.unf` `.unif` | Nestopia UE | GPL-2.0 | `libretro-nestopia` (Arch Linux ARM) |
| Famicom Disk System | `FDS` | `.fds` | Nestopia UE | GPL-2.0 | the same |
| Mega Drive | `MD` | `.md` `.gen` `.bin` `.smd` | BlastEm | GPL-3.0 | `libretro-blastem` (Arch Linux ARM) |
| Game Gear | `GG` | `.gg` | Gearsystem | GPL-3.0 | `libretro-gearsystem`, pinned 66fe790 |
| Master System | `SMS` | `.sms` | Gearsystem | GPL-3.0 | the same |
| SG-1000 | `SG1000` | `.sg` | Gearsystem | GPL-3.0 | the same |
| Atari Lynx | `LYNX` | `.lnx` | Handy | Zlib | `libretro-handy`, pinned bc55d46 |
| PC Engine | `PCE` | `.pce` | Beetle PCE Fast | GPL-2.0 | `libretro-beetle-pce-fast` (Arch Linux ARM) |
| PC Engine CD | `PCECD` | `.cue` (+ its tracks) `.chd` | Beetle PCE Fast | GPL-2.0 | the same |
| Neo Geo Pocket (Color) | `NGP` | `.ngp` `.ngc` | Beetle NeoPop | GPL-2.0 | `libretro-beetle-ngp`, pinned a50d5ac |
| WonderSwan (Color) | `WS` | `.ws` `.wsc` | Beetle Cygne | GPL-2.0 | `libretro-beetle-wswan`, pinned 4b01295 |
| Game & Watch | `GW` | `.mgw` | gw-libretro | Zlib | `libretro-gw`, pinned 91d599b |
| PlayStation | `PS` | `.chd` `.cue` (+ tracks) `.pbp` `.m3u` | PCSX ReARMed | GPL-2.0 | `libretro-pcsx-rearmed-nibos`, pinned c881679 |

Each core has a small package of nibcase's, `nibcase-<core>` (`nibcase-snes9x`,
`nibcase-nestopia`, `nibcase-blastem`, `nibcase-gearsystem`, `nibcase-handy`, `nibcase-gw`,
`nibcase-beetle-pce-fast`, `nibcase-beetle-ngp`, `nibcase-beetle-wswan`,
`nibcase-pcsx-rearmed`), which depends on the core's and registers it with DSlot: an alpm hook
runs `nibcase-test --sync`, which asks the installed core for its options and writes
`/usr/share/dslot/emulators/nibcase-<core>.toml`, with one `[systems]` entry for each system the
core plays. Pinned cores are built from `packaging/cores/<core>/PKGBUILD`. The frontend never
contains a core.

**Licences.** nibcase is MIT; each core keeps its own. **Snes9x is the only
non-commercial core**: it must never be part of any commercial distribution. Every other core
listed is free (GPL-2.0, GPL-3.0 or Zlib) and may be distributed with the usual obligations
(the core's source for the GPL ones).

**`.chd` and `.cue`** belong to PC Engine CD and PlayStation both: the folder decides which.

**RetroAchievements** (Casual) are earned for SNES, NES, Mega Drive, Game Gear, Lynx and
PlayStation, the systems checked on the device; [RetroAchievements](retroachievements.md) says how,
and what the others wait for.

## BIOS

BIOS files are the player's own; none is distributed. Put them in `BIOS/` at the root of either
card (`/mnt/roms/BIOS` or the second card's); nibcase uses the card that has the file, and
`--bios DIR` overrides the search.

| File | For | Required | MD5 | Notes |
|---|---|---|---|---|
| `lynxboot.img` | Atari Lynx | Yes | `fcd403db69f54290b51035d82f835e7b` | Exactly 512 bytes. |
| `syscard3.pce` | PC Engine CD | Yes | `38179df8f4ac870017db21ebcbf53114` (262,144 bytes) | A copy with a 512-byte copier header (262,656 bytes, md5 `ff1a674273fe3540ccef576376407d1d`) works as it is: the core skips the header. Never strip the player's file. |
| `disksys.rom` | Famicom Disk System | Yes | `ca30b50f880eb660a320674ed365ef7a` | 8,192 bytes. |
| `scph5501.bin` | PlayStation | No (recommended) | `490f666e1afb15b7362b406ed1cea246` | Without any PlayStation BIOS the core's own (HLE) plays most games, and the menu says which file to add. |
| `scph1001.bin` | PlayStation | No | `924e392ed05558ffdb115408c263dccf` | |
| `scph7001.bin` | PlayStation | No | `1e68c231d0896b7eadcad1d7d8e76129` | |
| `scph101.bin` | PlayStation | No | `6e3735ff4c7dc899ee98981385f6f3d0` | |
| `PSXONPSP660.bin` | PlayStation | No | `c53ca5908936d412331790f4426c6c33` | |

A system whose BIOS is required doesn't start without it, and says which file is missing and
where it looked. Checksums are libretro's documentation's (the Handy, Beetle PCE Fast and PCSX
ReARMed pages) and, for `disksys.rom`, the player's checked file.

## Notes by system

### SNES, NES, Mega Drive, Game Gear, Master System, SG-1000, Lynx, PC Engine, Neo Geo Pocket

Ordinary cartridges with the standard pad. NES starts with Nestopia's composite filter off, so
the game's own pixels are scaled by whole steps; the filter stays a core option. Lynx games made
to be held upright are turned upright (the core says so through libretro's rotation).

### WonderSwan

Some games are played holding the console upright. A game says so in its footer (bit 0 of the
byte 4 from the end; WSdev wiki, "ROM header"), and nibcase reads it before the core loads the
game (Beetle Cygne reads its Display Rotation only then): upright games start in Portrait,
filling more of the panel (144 x 224 at 3x), the others in Landscape. The game's Display Rotation
setting, at the game's or the console's level, overrides it.

### PC Engine CD

`.cue` with its `.bin` tracks (DSlot hides the tracks) or `.chd`; never zipped. Needs
`syscard3.pce`. Checked on the DS+ with the player's headed System Card and HuC's CD example.

### Famicom Disk System

`.fds` disk images; needs `disksys.rom`. Registered and its BIOS check tested; not yet played
(no free Disk System homebrew was found).

### Game & Watch

gw-libretro plays MADrigal's simulators converted to `.mgw`; the player supplies them. The core
draws a photograph of the whole unit with its LCDs live inside (`crates/nibcase/src/unit.rs`):

- **Whole unit (default).** A unit that opens on a hinge is cut at the hinge, lid on the top
  panel and body on the bottom, at one scale, meeting across the gap between the panels: Donkey
  Kong, Donkey Kong II, Mario Bros. and Lifeboat (Multi Screen; Mario Bros. and Lifeboat open like
  a book, left page on top), the five Panorama Screen units, and Penguin Land (two consoles side
  by side). Every other unit is drawn whole on the top panel, smoothly scaled to fit, with touch
  buttons on the bottom panel.
- **Screen only** (the Show setting). Units with one LCD are zoomed by the core itself (its Zoom
  key, reached by a number of Selects measured for each unit); the five units with two LCDs have
  them cut out, one per panel. **LCD in Screen only: Brighter** (default) lifts each LCD so its
  pale background is near white, a gain measured once per unit that keeps the segments dark; or
  As photographed.
- **Controls.** The D-pad, A and B are the unit's own controls (which ones a unit uses varies).
  **Select** moves the core's pointing hand to the unit's next key (Game A, Game B, Time, then the
  unit's others such as ACL and Zoom) and **Start** presses it. The touch buttons under a whole
  unit do the same (Next key, Press), and on Nintendo's units, whose keys come in that order, Game
  A / Game B / Time are one tap each, from the hand put away. The Play page lists the controls.
  Taps on a unit's painted buttons reach the core but these conversions ignore them.
- **Saves.** The core keeps no save states, so a game starts afresh; its 777 bytes of battery
  memory are kept as the game's `.srm`.

Hinge positions, LCD rectangles and Zoom places are measured per unit (name and frame size); a
different conversion of a unit is drawn whole, never cut wrongly.

### PlayStation

- **Discs:** `.chd`, `.cue` with its `.bin` tracks, `.pbp`; never zipped. A game of several discs
  is an `.m3u` listing them, one file name a line, beside them:

  ```
  Game (USA) (Disc 1).chd
  Game (USA) (Disc 2).chd
  ```

  It gets a **Disc** row on the Play page: A or Left / Right puts the next or previous disc in the
  drive (through libretro's disk control). DSlot shows the `.m3u` and hides the discs it lists.
- **Memory cards:** card 1 is the game's own, its battery memory, kept as `<game>.srm` in the
  game's saves folder; card 2 is the core's shared card, also in that folder.
- **BIOS:** optional (table above); without one the core's HLE BIOS plays, with a note in the menu.
- **Controller:** the Controller setting, **Digital pad** (default) or **DualShock**, the stick as
  the left analog stick, per game or for every PlayStation game, plugged again when changed. The
  DS+ has one stick.
- **Screen:** fitted at the television's 4:3 and smoothly scaled by default, since games change
  resolution (320 x 240, 512 x 240, 640 x 480) and their pixels aren't square. The bottom panel is
  blank in play.
- **Performance:** with Gran Turismo on the DS+ most frames take about 2 ms, but the 99th
  percentile is about 14.7 ms: occasional frames cost nearly a whole frame. Not yet profiled
  (disc reads and recompiling are suspected); a race not yet measured. Enhanced resolution is
  left off until this is understood. See [validation](validation.md).

## What every system shares

- **The menu** is nibkit's, as in DSlot, inkdot and DSperate: Play, States, Game, the console's
  defaults, System, and on nibOS Connections (nibkit's ready-made page: Wi-Fi and Bluetooth on or
  off, paired devices connected or disconnected with one press). **States** has the resume point as its own tile at the left, labelled with
  when it was written, then six numbered slots with their times; the page opens on slot 1, Left
  reaches the resume tile, and it can be loaded but never saved over.
- **Buttons:** Back opens and closes the menu; Menu saves and quits; Menu + R / L quick save and
  load; Menu + Select fast-forwards while held; Menu + volume is nibOS's brightness.
- **A game's saves folder is the game's, not the core's.** Every core that plays a system uses
  `Saves/<folder>/<game>/`, and the player may switch cores. Another core's resume state is moved
  aside to `<game>.auto-<core>.state` and comes back when that core plays again; a battery save of
  another size is loaded as far as it fits, the original kept as `<game>.srm.<size>.bak`; battery
  memory a core shows only after its first frame is loaded then, and never written before it is
  loaded. Cores without save states (gw-libretro) quit cleanly without a resume state.
- **Settings** have three levels (game, console, default) with the "Use …" rule; options a core's
  description declares live in DSlot's settings, the rest in nibcase's own file. A game's own
  defaults (a WonderSwan game's orientation) sit below any level the player set.
