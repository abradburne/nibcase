# Game & Watch through gw-libretro

The core is a separate package, `libretro-gw`, built from this directory's PKGBUILD at a pinned
commit (gw-libretro 1.6.3, zlib licence: fine for any distribution, commercial included).
Installing it with `nibcase-gw` refreshes its DSlot description; removing it removes only that
description.

The core plays MADrigal's Game & Watch simulators converted to `.mgw` files. No unit is
distributed: put the player's own `.mgw` files in `Roms/GW` on either card. No BIOS is needed.

nibcase draws the core's photograph of each unit. The nine units that open on a hinge (the
Multi Screen and Panorama Screen ones) are cut at the hinge across the two panels; the rest are
drawn whole on the top panel, with touch buttons for Game A / Game B / Time (Nintendo's units)
and Next key / Press (all) below. See `crates/nibcase/src/unit.rs`.
