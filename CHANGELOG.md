# Changelog

## 0.2.0

- **RetroAchievements** (Casual), through nibkit's `nib-cheevos` (nibkit 0.2.1), for SNES, NES,
  Mega Drive, Game Gear, Atari Lynx and PlayStation, each played on the RG DS+ to its set's
  placard (docs/retroachievements.md). DSlot passes its sign-in to the cores whose descriptions
  declare the system: `nibcase-test --sync` now writes `[achievements] systems = [...]` for
  those. The game is identified by rhash as DSlot identifies it (zips, discs from their cue
  sheet, `.m3u`, and `.chd` through nib-cheevos' new CHD reader); the set is checked after every
  frame, fast-forward's too, and kept alive while paused; reset and loading a state start the
  progress over or restore it, and states keep it. rcheevos' rules for libretro cores hold: a
  core it rules out for a console earns nothing, and an option it rules out (a PAL region, a
  layer turned off, an underclocked CPU) pauses the set with a notice. The placard, unlocks with
  their badges, the game completed and problems are nibkit's toasts over the game; progress and
  challenges its indicators. User-Agent `nibcase/0.2.0 (nibOS; linux aarch64) <core>/<version>
  rcheevos/12.4`. `NIBCASE_DEBUG=cheevos` logs the requests and how the console was mapped;
  `NIBCASE_PEEK=<address>` logs 4 bytes of the console's memory once a second.
- **Memory maps:** the host answers `RETRO_ENVIRONMENT_SET_MEMORY_MAPS` (kept, the core's pointers
  its own: `Core::memory_map`, `Core::memory_maps`) and gives a core's memory in place
  (`Core::memory_in_place`); `SET_SUPPORT_ACHIEVEMENTS` is accepted.
- **States:** version 2 adds a fourth section, RetroAchievements' progress; a state without it is
  written as version 1, as before, and both are read.
- nibkit 0.2.0 (from 0.1.0) and nib-cheevos; the package depends on `curl` (the requests) and
  installs rcheevos' licence.

## 0.1.0

The first release: a dual-screen libretro player for the Anbernic RG DS+ on nibOS, with the same
menu, buttons, saves and pacing as nibOS's other emulators and DSlot.

- **Systems:** SNES (Snes9x), NES and the Famicom Disk System (Nestopia UE), Mega Drive (BlastEm),
  Game Gear, Master System and SG-1000 (Gearsystem), Atari Lynx (Handy), PC Engine and PC Engine
  CD (Beetle PCE Fast), Neo Geo Pocket (Beetle NeoPop), WonderSwan (Beetle Cygne), Game & Watch
  (gw-libretro) and PlayStation (PCSX ReARMed with its ARM64 dynarec). Cores are separate
  packages, Arch Linux ARM's or built from upstream at pinned commits (`packaging/cores`).
- **DSlot integration:** a package per core; installing, upgrading or removing one asks the core
  for its options and writes or removes its DSlot description (`nibcase-test --sync`). A core
  that plays several systems is described for each.
- **Menu:** nibkit's, on the bottom screen: Play, save states (six slots and the resume point,
  each with its picture and time), the game's and the console's settings at three levels,
  System, and on nibOS Connections (Wi-Fi and Bluetooth on or off, paired headphones connected
  with one press). The top screen shows the game paused, its name fitted to the screen.
- **Saves:** battery memory and states in the game's own saves folder, shared by every core that
  plays the system; a resume state on quit and SIGTERM; states checked for their core, version
  and game.
- **Game & Watch:** units that open on a hinge cut across both screens, the others whole with
  touch buttons for their keys; Screen only shows the LCDs alone, brightened.
- **Disc games:** `.chd`, `.cue`, `.pbp` and `.m3u` for a game of several discs, changed from the
  menu; PlayStation's pad (digital or DualShock) and an optional BIOS with the core's own as a
  fallback.
- **Per system:** BIOS files found on either card and checked before a game starts; WonderSwan
  games played upright start upright; rotation for portrait games; Nestopia's composite filter
  off for whole-pixel scaling.
- **Sound and pacing:** 48 kHz windowed-sinc resampling with bounded queue correction; games near
  60 Hz follow the panel's 59.94 Hz.
- **Tools:** `nibcase-test` (`--describe`, `--sync`, `--bench` with `--state-check`, scripted
  `--press` / `--tap` and `--frame`), a C test core for the host, and generators for original
  NES and Sega test programs.
