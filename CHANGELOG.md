# Changelog

## Unreleased

- **PSP**, through PPSSPP (`nibcase-ppsspp`, with `libretro-ppsspp-nibos` built from PPSSPP's
  v1.20.4 tag for OpenGL ES with its ARM64 JIT): UMDs as `.iso`, `.cso` and `.chd`, PSN downloads
  as `EBOOT.PBP`, in `Roms/PSP`. No BIOS; the core's own files are linked into
  `~/.local/share/nibcase/PSP/PPSSPP/`. Shown at 2x, 960 x 544; the stick is the PSP's analog
  stick. RetroAchievements declared (the memory map checked; a game's placard on the DS+ still to
  see).
- **Hardware-rendered cores** (`SET_HW_RENDER`, OpenGL ES 2/3 and OpenGL): a context of the
  core's own through EGL with no window, made when the core asks and current only while it runs
  (SDL's is put back after every call), a framebuffer that grows with the core's frames, and the
  frames read back through two pixel-pack buffers into the usual frame, a frame late. EGL and GL
  are loaded only when a core asks. The player draws a GPU core's frames from the GPU: its colour
  buffer exported as a dma-buf (`Core::hw_export`) and imported as the top panel's texture
  (`Imported`), no frame copied; a frame is read back only for a picture (`Core::refresh_frame`).
  On Panfrost reading every frame back cost 10 ms at 1x. `NIBCASE_DEBUG=gl` logs the readback's
  parts, `NIBCASE_DEBUG=frames` the core's time a frame; the loop's error is said before the core
  is torn down. `GET_PREFERRED_HW_RENDER` answers OpenGL ES 3;
  `SET_HW_SHARED_CONTEXT` is accepted.
- **A GPU core's frames reach the screen without a dma-buf:** its context is made in the top
  panel's renderer's share group (`share_with_current_context`), and each frame is copied on the
  GPU into the panel's texture (`Core::hw_shared`, `copy_shared_frame`: a blit) after a fence. Importing
  an exported dma-buf into SDL's display, as before, made the display controller's driver fail to
  map it at every launch and, after about 80, every frame came out white until a reboot. The
  dma-buf export is gone.
- **PSP: framebuffer copies on the GPU.** Panfrost does glCopyImageSubData on the CPU, waiting for
  the GPU first; PPSSPP uses it for every effect drawn into a texture. nibcase hides the extension
  from it (`systems::CORE_ENV`, `MESA_EXTENSION_OVERRIDE`, set before the first GL context, by the
  player and nibcase-test), and the core blits on the GPU: a Ridge Racer 2 race 19.6 to 12.1 ms a
  frame, God of War's title 19.3 to 13.5, the picture the same. Found with perf on the DS+.
- **Leaving without C++'s static destructors:** once the game is saved and the core unloaded, the
  player ends with `_exit`. PPSSPP keeps a `std::thread` among its statics that can still be
  joinable at exit (Burnout Dominator's, once on the DS+); destroying it aborted the process after
  everything was saved, and DSlot said the game had crashed.
- **A core's game is unloaded before its GL context goes**, and a core that has unloaded its
  game isn't told the context goes: PPSSPP saves its shader cache when it unloads, and saved
  nothing when told first (its `PSP/SYSTEM/CACHE` stayed empty, so every launch compiled each
  shader again mid-game, a stop of up to half a second each time); told after the unload, it
  crashed. Now the cache is saved, and the next launch precompiles it while loading.
- **Resuming a core that boots on a thread of its own** (PPSSPP): it has no states just after
  the load, and the game didn't resume. The resume now waits after each frame until the core has
  states (ten seconds at most, then a notice), and quitting before then doesn't write over it.
- **Handheld defaults and hidden options** per core (`HANDHELD_DEFAULTS`, `HIDDEN_OPTIONS` in
  `systems.rs`): a default that isn't the core's is the default everywhere (the menu, Y, DSlot's
  description); a hidden option is in neither menu.
- nibkit 0.3.1 (from 0.2.1): PSP games hashed (with `cso`, CSO images); the menu's `sections`, unused.

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
- **Mega Drive buttons:** the DS+'s Y, B and A are the pad's A, B and C (C, Aladdin's and most
  games' jump, was on R with BlastEm), X, L and R its X, Y and Z, Select Mode. A small table per
  system (`layout.rs`) lays the DS+'s buttons out by the names the core gives its own (its input
  descriptors), so any Mega Drive core gets it; other systems keep the core's layout. Logged at
  load: `nibcase: buttons: Mega Drive pad: Y=A B=B A=C X=X L=Y R=Z Select=Mode Start=Start`.
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
