# Lynx through handy

The core is a separate package, `libretro-handy`. Installing it with `nibcase`
refreshes its DSlot descriptor from the actual core options. Removal removes only
that generated descriptor. No ROMs or firmware are distributed.

Handy needs your own 512-byte `lynxboot.img` in the card's `BIOS/` folder, or
a directory passed with `nibcase --bios DIR`. The core defaults to 60 Hz video
updates and supports portrait games through libretro rotation.

Build the pinned core with this directory's PKGBUILD on an aarch64 Arch Linux ARM machine, then
install `nibcase-handy` alongside it. Gameplay/performance validation is pending
a BIOS; it has not been measured on a real Lynx game yet.
