# PC Engine and PC Engine CD through Beetle PCE Fast

This package depends on Arch Linux ARM's `libretro-beetle-pce-fast` (GPL-2.0). The core stays a
separate shared library; nibcase keeps its own licence. Installing or upgrading it registers its
options with DSlot for PC Engine / TurboGrafx-16 HuCards (`Roms/PCE`: `.pce` or a `.zip` holding
one) and CD-ROM² discs (`Roms/PCECD`: `.cue` with its tracks, or `.chd`).

HuCards need no BIOS. CD games need your own System Card image, `syscard3.pce`, in `BIOS/` at
the root of either card; nibcase won't start a CD game without it. No firmware is distributed.
