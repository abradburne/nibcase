# nibcase

A dual-screen libretro player for the Anbernic RG DS+ on nibOS: libretro cores with the same
menu, buttons, saves and pacing as nibOS's other emulators (inkdot, DSperate) and DSlot. The
frontend is original MIT-licensed code; the cores are separate packages under their own
licences.

| System | Folder | Files | Core | BIOS (yours, in `BIOS/`) |
|---|---|---|---|---|
| SNES | `Roms/SFC` | `.sfc` `.smc` | Snes9x (**non-commercial licence**) | |
| NES | `Roms/FC` | `.nes` `.unf` `.unif` | Nestopia UE | |
| Famicom Disk System | `Roms/FDS` | `.fds` | Nestopia UE | `disksys.rom`, required |
| Mega Drive | `Roms/MD` | `.md` `.gen` `.bin` `.smd` | BlastEm | |
| Game Gear | `Roms/GG` | `.gg` | Gearsystem | |
| Master System | `Roms/SMS` | `.sms` | Gearsystem | |
| SG-1000 | `Roms/SG1000` | `.sg` | Gearsystem | |
| Atari Lynx | `Roms/LYNX` | `.lnx` | Handy | `lynxboot.img`, required |
| PC Engine | `Roms/PCE` | `.pce` | Beetle PCE Fast | |
| PC Engine CD | `Roms/PCECD` | `.cue` `.chd` | Beetle PCE Fast | `syscard3.pce`, required |
| Neo Geo Pocket (Color) | `Roms/NGP` | `.ngp` `.ngc` | Beetle NeoPop | |
| WonderSwan (Color) | `Roms/WS` | `.ws` `.wsc` | Beetle Cygne | |
| Game & Watch | `Roms/GW` | `.mgw` | gw-libretro | |
| PlayStation | `Roms/PS` | `.chd` `.cue` `.pbp` `.m3u` | PCSX ReARMed | `scph5501.bin` etc., optional |

Cartridges may also be a `.zip` holding one game; discs never are. Games may be on either card.
**[docs/systems.md](docs/systems.md)** has every system's details: cores and their licences and
packages, the BIOS files with their checksums, Game & Watch's dual-screen units and Screen only,
WonderSwan's upright games, the headed PC Engine System Card, and PlayStation's discs, memory
cards, controller and BIOS.

## Install

Install `nibcase` and the integration for each core you want, `nibcase-<core>`
(`nibcase-snes9x`, `nibcase-nestopia`, `nibcase-blastem`, `nibcase-gearsystem`, `nibcase-handy`,
`nibcase-beetle-pce-fast`, `nibcase-beetle-ngp`, `nibcase-beetle-wswan`, `nibcase-gw`,
`nibcase-pcsx-rearmed`), which pulls in the core's own package. Installing, upgrading or removing
one runs a package hook that asks the core for its options and writes or removes its DSlot
description; DSlot shows the system's page once it has games. Your own descriptions in DSlot's
folders still win. No game, BIOS or core binary is part of nibcase.

## Play

From DSlot, or by hand:

```sh
nibcase                                   # pick an SNES game from both cards
nibcase --system PS                       # pick a PlayStation game
nibcase "Roms/WS/Game.wsc"                # the system from the folder or the file
nibcase --core /usr/lib/libretro/snes9x_libretro.so --option snes9x_overclock_superfx=100% game.sfc
```

- **Back** opens and closes the menu on the bottom screen; a tap on the bottom screen opens it too
  (except where it carries a Game & Watch unit's touch buttons).
- **Menu** saves and quits. **Menu + R / L** quick-save / quick-load. **Menu + Select** fast-forwards
  while held. **Menu + volume** is nibOS's brightness.
- **The menu:** Play (Resume, Save / Load state, the Disc for a multi-disc game, Fast-forward,
  Start over, Save and quit), States (the resume point as its own tile, then six slots, each with
  its picture and time: A loads, X saves, X again over a used slot), Game and the console's page
  (settings for this game or every game of the system: Y goes back to the inherited value),
  System (volume, each screen's brightness, clock, battery protect), and on nibOS Connections
  (Wi-Fi and Bluetooth on or off, paired headphones connected or disconnected with one press;
  pairing and joining networks stay in DSlot).
- Face buttons follow their printed labels; the stick can be the D-pad (a setting), or a
  PlayStation DualShock's analog stick.

The top screen shows the game at a whole-number scale with square pixels, except Game & Watch
and PlayStation, which are fitted to the panel and scaled smoothly (PlayStation at the
television's 4:3). The bottom screen is dark in play unless a game uses it (Game & Watch). Games
near 60 Hz follow the panel's 59.94 Hz with their sound resampled; Exact speed keeps the game's own
rate. Sound is resampled to 48 kHz by a windowed-sinc filter; nothing runs on a real-time thread.

## Saves

A game's battery save (`.srm`, also PlayStation memory card 1), clock and states live in
`Saves/<folder>/<game>/` on the game's card; DSlot passes that folder (`--saves`). Battery memory
is checked once a second and written atomically. Menu and SIGTERM save a resume state, and the
game resumes from it next time (`--fresh` starts over, keeping the battery save). States carry
the core and its version, the game's identity, the play time and a picture; one from another core
or version is refused, never loaded wrongly.

The folder is the game's, shared by every core that plays the system: another core's resume
state is kept aside (`<game>.auto-<core>.state`) and comes back with that core, and a battery save
of another size is loaded as far as it fits with the original kept (`<game>.srm.<size>.bak`).
Details in [systems](docs/systems.md#what-every-system-shares).

## Build, check, measure

You need Rust, a C compiler and SDL2. nibkit, nibOS's SDK library, comes from
[its repository](https://github.com/abradburne/nibkit) at the tag `Cargo.toml` names. For the
nibOS packages, on an aarch64 Arch Linux ARM machine:

```sh
make check                                # fmt, clippy -D warnings, tests (test core included)
cd packaging/cores/<core> && makepkg      # a pinned core, where Arch's won't do
cd packaging/nibos && PKGDEST="$PWD/out" makepkg -f --nodeps
```

`nibcase-test` is the workbench: `--describe CORE` (the DSlot description), `--sync DIR` (what
the package hook runs), `--bench CORE GAME [--state-check] [--press BUTTON@N] [--tap X,Y@N]
[--frame PNG]` (cost per frame, state continuation, scripted input, the last frame).
`NIBCASE_DEBUG=input` logs which inputs a core reads. `--headless` runs the core without SDL; for
pictures without the panels use `SDL_VIDEODRIVER=dummy` and `--screenshot`.
Desktop: `cargo run -p nibcase --no-default-features --features desktop,dslot -- --core CORE GAME`.

**[docs/adding-a-core.md](docs/adding-a-core.md)** walks through adding a core or a system.
[docs/validation.md](docs/validation.md) records what was measured and what isn't checked yet;
[docs/provenance.md](docs/provenance.md) every source used. Contributing:
[CONTRIBUTING.md](CONTRIBUTING.md).

## Licence

nibcase is MIT ([LICENSE](LICENSE)). The cores are separate programs under their own licences,
installed as their own packages and never built into nibcase ([docs/systems.md](docs/systems.md)
lists them; **Snes9x's licence is non-commercial**). The fonts nibkit draws with are under the
SIL Open Font License 1.1; `libretro.h` is MIT ([THIRD-PARTY.md](THIRD-PARTY.md)). No game, BIOS
or core is part of this repository.

This release plays software-rendered cores only (no GPU cores yet), with the standard pad;
netplay, rewind, cheats, shaders and special peripherals are not part of it.
