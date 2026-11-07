# PSP through PPSSPP

The core is a separate package, `libretro-ppsspp-nibos`, built from this directory's PKGBUILD from
PPSSPP's release tag (GPL-2.0-or-later) for OpenGL ES 3 with its ARM64 JIT; Arch Linux ARM has
PPSSPP's own program but no libretro core. Installing it with `nibcase-ppsspp` registers its
options with DSlot for PSP games in `Roms/PSP`: UMDs as `.iso`, `.cso` or `.chd` (`chdman
createdvd`), and PSN downloads as `EBOOT.PBP`, on their own or in a folder each.

- **BIOS:** none. The core has the PSP's firmware functions itself; the PSP's fonts and the core's
  own files come with the package (`/usr/share/libretro/ppsspp/PPSSPP`), and nibcase links them
  into the core's system folder, `~/.local/share/nibcase/PSP/PPSSPP/`.
- **Saves:** the game's memory-stick saves (`PSP/SAVEDATA/`) are in its saves folder,
  `Saves/PSP/<game>/`, beside its states.
- **Picture:** the core draws with the GPU (OpenGL ES 3 through EGL, its own context) and nibcase
  reads each frame back. 480 x 272 is shown at twice its size, 960 x 544, with square pixels.
- **Controls:** face buttons by position (B is Cross, A Circle, Y Square, X Triangle), L and R, the
  D-pad, and the stick as the PSP's analog stick (Stick as D-pad is off for the PSP).
