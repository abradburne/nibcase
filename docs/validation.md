# Validation

What was measured, on what, and what isn't checked yet; the sections below, oldest first, give
the details. Costs are the core's per frame on the RG DS+ (p99), headless.

| System | Played with | On the DS+ | Not checked yet |
|---|---|---|---|
| SNES | a free test ROM; owned games | Snes9x 3.6 ms; started from DSlot | enhancement-chip games |
| NES, Mega Drive, Game Gear | free and original test programs | see their sections | real games' feel |
| Lynx | — | registered | gameplay (needs `lynxboot.img` and games) |
| Game & Watch | 59 owned `.mgw` units | 3.7-8.3 ms; played in hand | long-session battery |
| Master System | Stalactites (homebrew) | 2.8 ms | real games, the panels |
| PC Engine | HuC samples | 2.3 ms | real games, the panels |
| PC Engine CD | HuC's CD example, an owned System Card | 2.9 ms | real games |
| Neo Geo Pocket | Asteroids Neo (homebrew) | 3.9 ms | real games, the panels |
| WonderSwan | SwanSong Originals (homebrew) | 1.2-2.8 ms | a real upright game |
| Disk System | — | registered, BIOS check tested | gameplay (no free homebrew found) |
| SG-1000 | — | registered | gameplay (no free homebrew found) |
| PlayStation | an owned Gran Turismo, HLE BIOS | boots to car select; p50 2.1 ms, p99 14.7 ms | a race, the spikes' cause, multi-disc, DualShock, a real BIOS |

## The first SNES release, 2026-10-20

## Automated checks

`make check`: formatting, clippy with warnings denied, workspace tests, and desktop/stock feature
build checks. The host's original C fixture exercises all three software pixel formats,
padded pitch, duplicate frames, geometry changes, hardware-frame refusal, joypad mapping,
stereo single/batch audio, option visibility, serialization continuation, SRAM sizes, stored
and deflated ZIPs, and the one-core process guard. Frontend tests cover streaming resampling
(chunk continuity, DC gain, alias rejection and queue correction), state containers/corruption,
atomic save writing/errors, CLI validation, file stems, and integer/aspect geometry.

The actual registration CLI is tested against a compiled fixture: generated descriptions parse
with nibkit's DSlot parser, expose options under the correct id, remain untouched when unchanged,
use the installed core path, remove stale owned descriptors and preserve personal descriptors.

DSlot's separate tests cover SNES headers, ZIPs, artwork fitting, commands/save paths and registry
changes. Native nibOS and stock builds are checked independently. The registry is re-probed
every two seconds while the shelf runs, and compared again after games. An actual running
SDL dummy shelf was smoke-tested: adding an installed descriptor caused a refresh, and
removing it caused a second refresh without restarting the process.

## Real core / free test ROM

The Arch Linux ARM build VM and RG DS+ loaded Snes9x as a black-box library:

- Arch package `libretro-snes9x`, `1:20260919.102324.gfae2fea08f74-1`, aarch64.
- Core reports `Snes9x 1.63 fae2fea0`, 60.09881 Hz and 32040 Hz audio.
- Downloaded package SHA256: `a237edfef520e33da38bb3e8d037709871beb49ca1c399babf8405916a7c5766`.
- Package signature/integrity checked by pacman; package extracted in a temporary directory,
  without installing it or reading its emulator implementation.
- ROM: `hdmaen_latch_test.sfc` from Marcus Rowe's MIT-licensed
  [snes-test-roms v20210217 release](https://github.com/undisbeliever/snes-test-roms/releases/tag/v20210217).
  Release ZIP SHA256: `00945927bee494317663accaeb21d854b6cdfef674813e6c119877a8eeacb1c4`.
  ROM CRC32 reported by the core: `f5089993`. No ROM is included in this repository/package.

`nibcase-test --bench CORE ROM --frames 1200 --state-check` passed in the ARM VM: restoring a
serialized state produced identical subsequent video and audio. The full player ran 300
headless frames, wrote an automatic state, resumed it, and rendered play/pause PNGs through
SDL's dummy renderer.

On the **RG DS+**, 1800 headless frames passed the same state-continuation comparison.
Measured core execution for this simple test ROM: median **3.3923 ms**, p99 **3.6189 ms**.
These numbers include the host's frame copy and exclude display/audio output. They describe
this ROM, not general SNES compatibility or demanding enhancement-chip games.

The full SDL KMSDRM player ran **600 frames** on the two physical panels, followed by a paused
menu run and automatic resume. Whole process time was **11.198 s** (including startup, loading,
shutdown and saving), user CPU 4.687 s, system CPU 0.761 s. Screenshots read before presentation
show the 256×224 test pattern at 3× scale on top and a black bottom in play, and the shared menu
on bottom while paused. Its menu pixels, excluding the device status bar, match the dummy
preview exactly. DSlot was idle before the test, stopped briefly, and restored automatically.
Temporary binaries/settings were staged under `/tmp/nibcase-validation`; installed software
was not changed. A separate on-device `dslot --launch` smoke test read a generated descriptor, passed
`--option snes9x_aspect=uncorrected` and the real `{saves}` folder to nibcase, and completed
120 headless frames with exit 0. Its newly created test save folder was checked and removed.
The running DSlot service remained active.

The test ROM is silent; this does not validate audible speaker/headphone output.

The core requests VFS, subsystem and achievements interfaces that this release returns false
for; Snes9x successfully falls back to the software/single-cartridge path. These appear once
in the log as unsupported environment calls. No BIOS is required for ordinary SNES cartridges;
optional enhancement-chip firmware remains the core's responsibility.

An automated SIGTERM test of the full player also exited 0 and flushed a nonempty automatic
resume state. The held-power/power-menu interaction remains a physical check.


## Remaining hands-on checks

- Physical buttons, touch targeting, Menu chords, external controllers/hotplug.
- Audible speakers/headphones and switching output, especially fast-forward and PAL.
- Suspend/wake recovery and held-power SIGTERM through the power menu.
- Representative commercial/homebrew games, PAL, copier-header ROMs and enhancement chips.
- Sustained pacing, power/thermal behavior.

A fixture validates host behavior; neither it nor a hardware glitch test establishes game
compatibility. Installed firmware and existing saves were left alone.

## NES / Nestopia (2026-10-20)

NES uses `FC` and the separately packaged Nestopia core. The core was downloaded with pacman's
signature/integrity verification, extracted and run as a black box; no emulator implementation
was read. Package: `libretro-nestopia` `1:20260925.192050.g8f00f500912a-1`, aarch64; SHA256
`6498ed23468b6e2e551b2b49d87a87bdc2f5640da9624eaa6ce7f872b97ff0c0`. It reports
`Nestopia 2.0.0 8f00f50`, NTSC 60.09881 Hz and 48 kHz audio. Its XML database is baked in;
ordinary cartridges need no external BIOS.

`tools/nes-smoke.py` builds an original mapper-0 cartridge with a checkerboard, a quiet pulse
wave, a persistent SRAM frame counter and a controller snapshot. No commercial ROM was copied
or bundled. `nes-check` runs it against the core and asserts nonzero audio, 256×224 video,
A/B/Start/Right input bits, 8192-byte SRAM restore, and exact video/RAM continuation after a
state restore. It passed in both the native ARM VM and on the RG DS+:

```sh
python3 tools/nes-smoke.py /tmp/Smoke.nes
CARGO_TARGET_DIR=/tmp/nibcase-target cargo run -p nibcase-test --release --example nes-check -- CORE /tmp/Smoke.nes
nibcase-test --bench CORE /tmp/Smoke.nes --frames 3600 --option nestopia_blargg_ntsc_filter=disabled
```

On the **RG DS+**, 3600 benchmark frames took **17.229 s**: median **4.6581 ms**, p99
**5.8685 ms**, including host frame copying and excluding display/audio output. This is the
simple test cartridge, not a compatibility/performance claim for every mapper or game. The
full SDL KMSDRM player then ran **600 frames**, resumed into the NES pause menu and returned to
DSlot. Whole play process: **11.227 s**, user CPU 4.988 s, system CPU 0.887 s (startup/saving
included). An isolated generated descriptor also launched 120 headless frames through DSlot
with `--system FC`, exit 0. All device saves/settings were under a temporary folder; DSlot was
restored and the temporary folder removed afterward. No packages were installed and no
existing saves or ROMs were changed.

VM checks also loaded a deflated `.nes.zip` with a readme before the cartridge. A forced-PAL
full-player run exercises the core's 50 Hz path. DSlot's NES cart/portrait cover preview and the
player's integer-scaled checkerboard were rendered and inspected. Automated tests cover NES
file naming/console inference, excluded disk/music extensions, descriptor generation/removal,
option namespaces/defaults, shared save paths and the new cart shell. `make check` builds and
tests nibcase with its target directory outside the checkout; nibOS/stock DSlot checks run
separately.

**State diagnostic limitation:** Nestopia does not reproduce bit-identical audio output after
unserialize in this test: the strict `--state-check` reports differing sample counts/content.
It remains strict and reports the mismatch; it was not changed to call this a pass. Video and
SRAM continuation match exactly with the composite filter disabled. With its composite filter
enabled even filtered-video phase can differ after restore. nibcase starts Nestopia with that
filter disabled for integer scaling and frugal rendering; the selectable filter and its
nibcase default agree in both DSlot and the player.

Physical controls, speaker/headphone listening, suspend and a representative set of mapper,
PAL and homebrew games remain hands-on checks. Disk System, music and special peripherals are
outside this first NES integration.

## Mega Drive / Game Gear / Lynx (2026-10-20)

Supported IDs now include `MD`, `GG`, `LYNX`. The package hook queries and manages all five
core descriptors, with fixture tests of creation, update, system/option scope, removal and
preservation of personal descriptors. C-stdio diagnostics are redirected to stderr for
`--describe`, with a noisy fixture regression test, so core logging cannot corrupt TOML. The player infers systems from folders, extensions and
core filenames; ZIP stems share the same save names. BIOS selection tests cover either card,
explicit overrides and missing firmware. Rotation tests exercise the environment callback,
padded source frames, all three quarter turns, display geometry/aspect and pixel placement.
Existing cross-core resume, battery-size backup and late-memory tests remain unchanged.

Sources/core packages:

- BlastEm: Arch Linux ARM `libretro-blastem` `20260926.203644.g1e0de94dc7e6-1`, aarch64,
  downloaded and signature/integrity verified by pacman, extracted without installation.
  Package SHA256 `5c9e5faaff1024f6684be67ff601c781a8bda893a9d36314f519e9b4cb3a2337`.
  Reports `BlastEm 1.0.1-pre`, 59.92274 Hz, 53267 Hz audio, default six-button pads.
- Gearsystem: upstream commit `66fe79082349e1365af3ac9fd26ea526b3cc7233`, built native
  aarch64 with `-mcpu=cortex-a55`, separate `libretro-gearsystem` package. Reports version
  `66fe790`, 59.92274 Hz, 44100 Hz audio, 160×144 output for the original Game Gear test ROM.
- Handy: upstream commit `bc55d462f0b2d6b073ea93dc552ebd73cec60fd1`, likewise built native
  aarch64 as a separate package. Reports `Handy 0.97 bc55d46`. Its real core options and
  `LYNX` descriptor generation were verified in the VM and on the device without loading a game.

`tools/sega-smoke.py` generates original 32 KiB colour-cycling programs. They exercise CPU,
VDP and frame timing, not demanding games, input coverage or audio hardware. No owned games
for these systems were at hand; no ROMs were copied.

| RG DS+ headless, 1800 frames after 120 warm-up frames | Median | p99 |
|---|---:|---:|
| BlastEm, original Mega Drive smoke, 320×224 | 8.5951 ms | 8.8372 ms |
| Gearsystem, original Game Gear smoke, 160×144 | 2.2592 ms | 2.4094 ms |

Both fit the frame budget for these smoke programs. This is not a representative-game or
sustained thermal benchmark. Gearsystem's state continuation matches video and audio exactly.
BlastEm passes the same check at 600 measured frames; at 1800 frames the restored video
matches, but ten subsequent frames produce 17808 versus 17702 audio samples, with all common
samples equal. The same discrepancy reproduces in the VM. The strict test still reports this
failure; no comparison was weakened. Treat exact audio continuation as a core limitation.

Both full players completed fresh/resumed 300-frame headless runs, then 180-frame play and
five-frame paused-menu runs through SDL KMSDRM on the physical panels. Captured images were
visually checked: 320×224 Mega Drive at 3× integer scale and 160×144 Game Gear at 5×, with the
shared pause menu and correct system label. Nonempty automatic states were written and reopened.
Temporary package-extracted binaries, settings and saves were under `/tmp/nibcase-next`.
The audio driver was dummy, so no sound was played. DSlot was restored by the cleanup trap.

**Lynx gameplay is not yet verified:** no `lynxboot.img` was found on either card or in the
checked stock firmware location. Missing/incorrect-size
firmware reports a clear error before the core loads a game. No replacement firmware was
invented or downloaded. Handy remains a candidate awaiting BIOS-backed device measurement.
Lynx portrait-game input, real games, save/resume, sound and performance remain to be checked.

`make check` passes in the ARM VM with an external target directory. Separate Gearsystem,
Handy and frontend/integration Arch packages build successfully. DSlot's native and stock
clippy/tests pass (60/61 tests, including the concurrent ROM-rescan work), and its three new
shelves were rendered with named fake games and inspected. All ten tabs fit without overlap.
No permanent device package installation was performed.

## Game & Watch (2026-10-21)

Core: gw-libretro 1.6.3 at 91d599b, built pinned (`packaging/cores/gw`), zlib. Units: 59
owned `.mgw` files (MADrigal's simulators, converted), read in place, not in this repository.

- **Cost on the RG DS+** (`nibcase-test --bench`, 1800 frames headless, p50 / p99 ms): Parachute
  3.44 / 3.67, Donkey Kong (Multi Screen) 5.94 / 6.41, Mario Bros. 6.68 / 7.18, Pac Man
  (Tomytronic) 7.94 / 8.34. The core redraws the whole photograph of the unit each frame.
- **In play on the panels** (900 frames, sound off, CPU of the nibcase process over 4 s): Donkey
  Kong cut across both panels 65% of one core, Parachute whole with its touch card 47%,
  Parachute screen only 38%. All exited 0. Screenshots checked: the hinge cut, the card, the
  zoomed LCD.
- **Layouts:** hinge positions and the two-LCD rectangles measured on raw frames
  (`--frame`); the Zoom key's place measured on every unit (`--press`); 54 units zoom, the other
  5 have two LCDs. Screenshots of each layout checked on the build machine (SDL dummy driver).
- **Not checked yet:** physical buttons and touch on the device, sound, a long session's battery
  draw, units other than the ones above in play, and DSlot's Game & Watch page.

## Master System, PC Engine, Neo Geo Pocket, WonderSwan, Disk System (2026-10-21)

Homebrew only (see provenance); no owned games for these systems were at hand.

- **On the RG DS+** (`nibcase-test --bench`, 1800 frames headless, p50 / p99 ms; cores built at
  the packaged commits): Master System, Stalactites (Gearsystem) 2.64 / 2.81; PC Engine, HuC shmup
  sample (Beetle PCE Fast) 1.87 / 2.33; Neo Geo Pocket, Asteroids Neo (Beetle NeoPop) 3.73 / 3.90;
  WonderSwan, Rotate Dungeon (Beetle Cygne) 0.94 / 1.18 and One Last Lap 0.96 / 2.77.
- **State continuation** (`--state-check`): identical video and audio for Master System, Neo Geo
  Pocket and WonderSwan on the device; PC Engine passed on the build machine, but its device run
  printed no result, so it is unconfirmed there.
- **WonderSwan orientation:** a landscape game starts across; a copy with its footer's upright
  bit set starts upright (144 x 224 at 3x), checked in player screenshots on the build machine.
- **PC Engine CD, on the RG DS+** with an owned `syscard3.pce` (read in place; it carries a
  512-byte copier header, which the core skips itself: a headed and a stripped copy boot the System
  Card to identical frames on the build machine) and HuC's CD example (`overlay.cue`, built from
  HuC's sources): the System Card loads the disc's first program; 352 x 240, 2.19 / 2.89 ms.
- **State continuation, PC Engine on the device:** the picture continues identically; the sound
  is off by about 2 samples in 10 frames (the core emits a fractional 737.1 samples a frame and
  doesn't keep its running fraction in the state). Harmless; on the build machine it matched.
- **Not checked:** the Disk System and SG-1000 (no free homebrew found);
  real games; the panels, buttons and sound on the device; DSlot's pages for these systems.

## PlayStation (2026-10-21)

Core: PCSX ReARMed at c881679 (r26), built pinned with the ARM64 dynarec (ari64). Game: an
owned Gran Turismo (USA) (Rev 1), a 410 MB `.chd` on the SD card, read in place; no BIOS (the
core's own, HLE).

- **On the RG DS+**, headless (`nibcase-test --bench`): from boot through the intros, 3600 frames,
  p50 2.14 ms, p99 14.79 ms; through the menus into Arcade mode's car selection (3D car drawn),
  4200 frames, p50 2.05 ms, p99 14.60 ms. The 99th percentile is near a whole frame: some frames
  cost much more than the rest (disc reads from the CHD and recompiling are the likely cause; not
  yet profiled). nibcase's pacer starts such frames before the vblank.
- **Not reached by scripted presses:** a race (a 9000-frame run ended on a loading screen). Race
  cost, the panels, buttons, sound, a two-disc `.m3u`, the DualShock and a real BIOS are still to
  be checked in play.

