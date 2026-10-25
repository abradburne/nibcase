# Changelog

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
