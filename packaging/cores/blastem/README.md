# Mega Drive through blastem

The core is a separate package, `libretro-blastem`. Installing it with `nibcase`
refreshes its DSlot descriptor from the actual core options. Removal removes only
that generated descriptor. No ROMs or firmware are distributed.

Uses Arch Linux ARM's `libretro-blastem`. Ordinary cartridges require no BIOS.
CD and 32X content are outside this integration. The current core restores the test
video but does not always reproduce the exact audio sample count after state load;
see the validation record. Smoke performance is not a demanding-game benchmark.
