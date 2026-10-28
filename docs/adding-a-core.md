# Adding a core or a system

How a core joins nibcase, from choosing it to the DS+. Each step names the file it touches; see
[systems](systems.md) for what is there now and [CONTRIBUTING.md](../CONTRIBUTING.md) for the rules.

## 1. Choose the core

- **Licence first.** Read the core's own licence file, not a package's metadata (Arch Linux ARM's
  once called two non-commercial cores BSD). Prefer free cores. A non-commercial one is flagged
  as such everywhere (as Snes9x is) and is never part of any commercial distribution.
- **Where it comes from.** Use Arch Linux ARM's `libretro-<core>` when it is current; otherwise
  build it pinned (step 4). Check what Arch's package installs: the library's name in
  `/usr/lib/libretro/` (`<name>_libretro.so`) is the core's name everywhere in nibcase.
- **Black box only.** Read the core's licence and build files, never its implementation; record
  every source in [provenance](provenance.md).
- **Measure before choosing it as a default** (step 6): bsnes was dropped for running a SNES game
  at 77 fps headless.

## 2. The system

In `crates/nibcase-libretro/src/systems.rs`:

- A `System` variant, in `ALL`, with its `id` (the stock OS's folder name under `Roms/`, which is
  also its key in DSlot), `label`, `extensions` (lower case, without the dot; `.zip` is accepted for
  cartridges by itself), and where they apply `bios` (required), `optional_bios`, `discs` (never
  zipped) and `has_rom_defaults` / `rom_defaults` (an option the game's own bytes decide).
- A `CoreSpec` in `CORES`: the core's name and every system it plays, the system it is inferred
  for first. A system's default core is the first entry that plays it.
- Tests in the same file: names that infer the system, the core that plays it.

A new system usually needs nothing in the player. Look for the cases that do: settings with
other defaults (`crates/nibcase/src/settings.rs`: Game & Watch and PlayStation fit the panel),
a layout of its own (`crates/nibcase/src/unit.rs`), a pad to plug (`plug_pad`), discs (the Disc
row comes by itself from libretro's disk control).

## 3. Registration with DSlot

- `crates/nibcase-test/tests/registration.rs`: add `(core, system)` to the list, and any other
  systems it plays.
- `packaging/nibos/PKGBUILD`: a `package_nibcase-<core>()` that depends on `nibcase` and the core's
  package and installs `packaging/cores/<core>/README.md`; add it to `pkgname` and `optdepends`;
  bump `pkgrel`.
- `packaging/nibos/90-nibcase-cores.hook`: a `Target` for the core's library, and its description
  in the `rm -f` list (used when nibcase-test is gone).
- `packaging/cores/<core>/README.md`: what it plays, the files, the BIOS, its licence.

Nothing else: on install the hook runs `nibcase-test --sync`, which writes the description from
the core's own options.

## 4. A pinned core (when Arch's won't do)

`packaging/cores/<core>/PKGBUILD`, as the others: `pkgname=libretro-<core>` (or `-nibos` with
`provides`/`conflicts` when it replaces Arch's), the upstream repository at a fixed `#commit=`,
`make` with `platform=unix` (and `-mcpu=cortex-a55` where the Makefile takes `CC`), the library
to `/usr/lib/libretro/`, the licence file to `/usr/share/licenses/$pkgname/`. Build it with
`SRCDEST` and `BUILDDIR` outside the repository so no clone lands in it.

## 5. Checks

```bash
make check    # on an aarch64 Linux machine, or any Linux for the desktop and stock checks
```

## 6. Measure

All with `nibcase-test`, on free homebrew or test programs (never committed), or games you own,
read in place:

```bash
nibcase-test --describe CORE                       # the description DSlot would get
nibcase-test --bench CORE GAME --system ID --frames 1800 --state-check
nibcase-test --bench CORE GAME --system ID --press Start@600 --tap 120,40@900 --frame out.png
NIBCASE_DEBUG=input nibcase-test --bench ...       # which inputs the core reads, once each
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy nibcase --core CORE --system ID --saves /tmp/s \
    --frames 120 --screenshot shot.png GAME        # the player's picture of both panels
```

`--bench` prints the core's cost per frame (p50, p99) and what each button does where the core
says; `--state-check` checks that a state continues identically. Run the same on the RG DS+ from a
folder in `/tmp` (no install needed), with saves in a temporary folder of their own, and remove
it after.

## 7. DSlot and the device

- **DSlot** needs a page for a new system: its folder, files and any scan rule (an `.m3u`'s discs,
  a `.cue`'s tracks hidden). That is a change to [DSlot](https://github.com/abradburne/DSlot):
  it needs the `[systems]` key, the files and the description's name.
- **On the device:** the packages land in `packaging/nibos/out/`; pinned cores are built
  separately (step 4). Install them with `pacman -U`.

### RetroAchievements

rcheevos maps every console's memory over a core's (its memory map or its memory ids), so a new
core needs no code for it; it needs a check. Play one game with a set on the device, with DSlot's
RA environment and `NIBCASE_DEBUG=cheevos`, and see the placard (and for a 16-bit console, that the
core's byte order is the service's: `NIBCASE_PEEK`). Then add the system to `System::achievements`
(the description declares it) and to [RetroAchievements](retroachievements.md).

## 8. Write it down

[systems](systems.md) (the table, the BIOS, the notes), [validation](validation.md) (what was
measured, on what, and what isn't checked), [provenance](provenance.md) (every source),
`CHANGELOG.md`, the README's table. Commits are signed off (CONTRIBUTING.md).

## What cores have taught us

Each of these cost a bug on the device; each now has a test.

- **A game's saves folder is the game's, not the core's.** DSlot passes
  `Saves/<SYSTEM>/<ROM name>/` to whichever core plays it, and the player can switch cores. A
  resume state made by another core is moved aside to `<stem>.auto-<core>.state` and comes back
  when that core plays again; it is never overwritten (`Saves::settle_resume`).
- **Cores disagree on battery RAM size** for the same game. A `.srm` of another size is loaded
  as far as both reach, and the original is kept as `<stem>.srm.<size>.bak` before anything is
  written (`fit_battery`, `battery_backup`).
- **Some cores show their memory late**: `retro_get_memory_size` is 0 until a frame has run
  (bsnes did). Battery files wait and are loaded after the first frame that shows the memory;
  until then that memory is never written, or the player's save is replaced by a blank one
  (`Saves::restore_pending`; testcore's mode 6).
- **Unsupported environment calls are logged once each** (`unsupported environment call N`):
  the journal shows what a core asks for. Snes9x asks for VFS, subsystems and achievements and
  falls back by itself.
- **A core may read the pointer on any port**: gw-libretro reads it on port 2, so the pointer
  answers every port (`NIBCASE_DEBUG=input` lists what a core reads). Reading it doesn't mean
  acting on it: those Game & Watch conversions ignore taps.
- **Some cores keep no states** (gw-libretro's serialize size is 0): check
  `Core::has_states()` before saving or resuming one, or quitting fails.
- **A core's own menus can be driven**: gw-libretro's Select / Start hand reaches a unit's keys
  and its Zoom; nibcase scripts them (`unit.rs`), with the places measured per unit.
- **Some options are read only at load**: Beetle Cygne reads Display Rotation when the game
  loads, so a WonderSwan game's orientation is taken from its bytes (`nibcase_libretro::peek`)
  and set before loading. nibkit's `Settings::value` falls back to the default by itself: ask
  `Settings::source` whether any level set an option before putting a game's default in.
- **Discs are big**: a PlayStation `.chd` is hundreds of MB. A core that reads the file itself
  gets the path, and the game is known by its first 64 MiB and its size.
- **Disk control's index of the disc count means an empty drive** (libretro.h): a core accepts
  it, so `Core::insert_disc` refuses it first.
- **Try the player's BIOS as it is**: their `syscard3.pce` has a 512-byte copier header, and the
  core skips it itself. Never modify a player's file.
- **Package metadata can be wrong about licences**: read the core's own licence file (Arch Linux
  ARM once listed two non-commercial cores as BSD).
- **Measure a core before making it the default**: `nibcase-test --bench CORE ROM` on the
  device. bsnes ran Donkey Kong Country at ~77 fps headless, too slow to play, and was dropped.
