# Nestopia for nibcase

This package depends on Arch Linux ARM's `libretro-nestopia`. The GPL-2.0-or-later core remains
an independent shared library; nibcase keeps its own licence. No emulator binary is vendored.

Installation/upgrade queries the actual library and registers its options and `FC` launch
command with DSlot. Put `.nes` (iNES/NES 2.0), `.unf`, `.unif` or single-cartridge `.zip` files in
`Roms/FC` on either card. Saves use `Saves/FC/<ROM stem>/`, shared by cores for that game.

It is also registered for the **Famicom Disk System**: `.fds` disk images in `Roms/FDS`, with the
your own `disksys.rom` (the Disk System's 8 KiB BIOS) in `BIOS/` at the root of either card; nibcase
won't start an FDS game without it.

The NES integration targets the standard joypad and cartridge games. NSF music, light guns and multitaps are outside this integration. The core’s audio output is not bit-identical after state restore in the current validation;
video/RAM state continuation passes. See nibcase’s validation record. No BIOS is required for
ordinary NES cartridges. See the [core's documentation](https://docs.libretro.com/library/nestopia/).
