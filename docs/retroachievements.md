# RetroAchievements

nibcase earns [RetroAchievements](https://retroachievements.org) in Casual (softcore) for the
systems below, through nibkit's `nib-cheevos`: rcheevos, the service's own library, for the
session, the game's identity and the console's memory. It shows what happens the way every
nibOS emulator does (nibkit's toasts and indicators), and logs it on stderr, where DSlot's bug
report picks it up. Code: `crates/nibcase/src/achievements.rs`.

| System | Earns | Checked on the DS+ with (placard) |
|---|---|---|
| SNES | yes | Donkey Kong Country: Casual, 0 of 71 unlocked |
| NES | yes | Contra: 0 of 40; Super Mario Bros. 3: 0 of 89 |
| Mega Drive | yes | Aladdin: 0 of 21; Sonic the Hedgehog: 0 of 35 |
| Game Gear | yes | Crystal Warriors: 0 of 73 |
| Atari Lynx | yes | California Games: 0 of 14 |
| PlayStation | yes | Gran Turismo (a `.chd`): 0 of 68 |
| Master System, Neo Geo Pocket, WonderSwan, PC Engine | not yet | only homebrew to hand, which the service doesn't know: signed in, identified, the memory mapped, "does not recognise this dump" |
| SG-1000, Disk System, PC Engine CD | not yet | no game with a set to hand |
| Game & Watch | no | rcheevos 12.4 has no hash rule for `.mgw` units |

A system earns once a game of it has been played to its set's placard; until then the core's
DSlot description leaves it out. The memory and hashing are rcheevos' for every system, so the
rest should need only that check: play one game with a set (DSlot's command, with its RA
environment; `NIBCASE_DEBUG=cheevos`), see the placard, and add the system to
`System::achievements` in `crates/nibcase-libretro/src/systems.rs`.

## The sign-in

DSlot signs in (the password never reaches nibcase) and starts a game with `RA_TOKEN_FILE` (the
token file: the username, then the token), `RA_USERNAME` and `RA_HARDCORE`, only for the systems a
core's description declares: `nibcase-test --sync` writes `[achievements] systems = [...]` for the
systems above. Without `RA_TOKEN_FILE` there is no session and nothing is logged. Hardcore isn't
offered yet (the service hasn't approved nibOS's emulators): with `RA_HARDCORE=1` nibcase says so
and plays Casual.

The User-Agent names nibcase and the core: `nibcase/0.2.0 (nibOS; linux aarch64) snes9x/1.63
rcheevos/12.4` (the core's file name and the first word of its version).

## The game

The game is identified as DSlot identifies it, by rhash (`nib_cheevos::hash::file`): a zip is its
first entry with one of the console's extensions, a cartridge's copier header is left out by
rhash's own rule, a disc is read from its cue sheet, a `.m3u` from its first disc, and a `.chd`
through nib-cheevos' CHD reader. DSlot's shelf and nibcase therefore agree on a game's hash.

## The console's memory

rcheevos lays out each console's memory in the service's address space
(`src/rcheevos/consoleinfo.c`), and `rc_libretro` reads it over a core's own: through the core's
memory map (`RETRO_ENVIRONMENT_SET_MEMORY_MAPS`, which nibcase now answers and keeps) when it gives
one, else through `retro_get_memory_data` / `retro_get_memory_size`. nib-cheevos' `libretro`
module is the Rust face of it. The regions are built over the core's memory in place, and built
again whenever the core gives a new map or a memory moves or changes size (some show their battery
RAM only after a frame). What each core gave on the DS+:

| Core | How | Regions |
|---|---|---|
| Snes9x | memory ids | work RAM 128 KiB, the cartridge's RAM (2 KiB for Donkey Kong Country) |
| Nestopia | memory ids | 2 KiB of RAM (and its mirrors), the cartridge's RAM when it has some |
| BlastEm | memory ids | 64 KiB of work RAM, the cartridge's RAM when it has some |
| Gearsystem | memory ids | 8 KiB of RAM, 32 KiB of cartridge RAM |
| Handy | memory ids | 64 KiB |
| PCSX ReARMed | memory map | 2 MiB of RAM, the scratchpad |
| Beetle PCE Fast | memory map | 8 KiB (PC Engine); with the CD's RAMs and the battery RAM (CD) |
| Beetle NeoPop, Beetle Cygne | memory ids | 16 KiB; 64 KiB |

BlastEm gives the Mega Drive's RAM as the service expects it, each 16-bit word's bytes swapped:
Sonic the Hedgehog's game mode (0x04 on the title screen, at the 68000's $FFF600) reads at the
service's $F601, which is where the game's set reads it (`NIBCASE_PEEK=F600` logs 4 bytes at an
address once a second, for a check like this).

## Play

- The set is checked after every frame the core runs, fast-forward's too; while the menu is open
  the session is kept alive (`rc_client_idle`).
- Reset starts the set's progress over. A save state keeps the progress (a version 2 state, its
  fourth section rcheevos' bytes; a state without a session is version 1 as before), and loading
  one restores it, or starts over for a state without any, so a jump in the machine's memory
  can't unlock anything. The resume point too.
- rcheevos' rules for libretro cores hold: a core it rules out for a console earns nothing there
  (`rc_libretro_is_system_allowed`), and while one of the core's options has a value it rules out
  (`rc_libretro_is_setting_allowed`: a PAL region for Snes9x or PCSX ReARMed, a graphics layer
  turned off, PCSX ReARMed's CPU underclocked) the set isn't evaluated, and a notice says why
  ("Achievements paused -- Console Region (Reload Core): PAL is not allowed"). Options are checked
  at the start and whenever play resumes from the menu; put back, the progress starts over.
- Shown over the game on the top screen: the placard as play begins (the game's title, "Casual:
  12 of 40 unlocked, 3 unsupported"), unlocks with their badges (fetched into the badge cache
  DSlot shares), the game completed, problems (a refused sign-in says to sign in again in DSlot);
  bottom left, the progress of a measured achievement and the badges of challenges that can be
  earned now. Leaderboards run only in hardcore: logged, not shown.
- Offline: rcheevos holds an unlock it couldn't send and retries it ("Offline", then
  "Reconnected"). It isn't kept past the session: nib-cheevos has no outbox on disk yet.

## Logs

`cheevos: <what> -- <detail>`, as DSperate and inkdot log them: `cheevos: Donkey Kong Country --
Casual: 0 of 71 unlocked`, `cheevos: Achievement unlocked: ... (5 points) -- ...`.
`NIBCASE_DEBUG=cheevos` adds every request, rcheevos' own messages, the game's hash and how
`rc_libretro` mapped the console.
