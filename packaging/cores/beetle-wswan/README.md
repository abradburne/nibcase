# WonderSwan through Beetle Cygne

The core is a separate package, `libretro-beetle-wswan`, built from this directory's PKGBUILD at a
pinned commit (GPL-2.0). Installing it with `nibcase-beetle-wswan` registers its options with DSlot
for WonderSwan and WonderSwan Color games: `.ws`, `.wsc` or a `.zip` holding one, in `Roms/WS`. No
BIOS is needed.

Some WonderSwan games are played holding the console upright. Each game says so in its footer,
and nibcase starts it that way (Display Rotation: Portrait), the others across (Landscape). The
game's Display Rotation setting overrides it.
