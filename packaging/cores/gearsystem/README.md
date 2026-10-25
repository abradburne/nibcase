# Game Gear and Master System through gearsystem

The core is a separate package, `libretro-gearsystem`. Installing it with `nibcase`
refreshes its DSlot descriptor from the actual core options. Removal removes only
that generated descriptor. No ROMs or firmware are distributed.

Build the pinned core with this directory's PKGBUILD on an aarch64 Arch Linux ARM machine, then
install `nibcase-gearsystem` alongside it. It is registered for Game Gear (`Roms/GG`, `.gg`) and
Master System (`Roms/SMS`, `.sms`) and SG-1000 (`Roms/SG1000`, `.sg`). Ordinary cartridges
require no BIOS.
The optional core boot ROM and link-cable features are not required by this integration.
