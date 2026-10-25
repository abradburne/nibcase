//! The systems nibcase plays (cartridge consoles and Game & Watch), shared by the player and
//! the DSlot descriptor generator.
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum System {
    Snes,
    Nes,
    MegaDrive,
    GameGear,
    Lynx,
    /// Nintendo's Game & Watch, as MADrigal's simulators converted to `.mgw` (gw-libretro).
    GameAndWatch,
    MasterSystem,
    /// PC Engine / TurboGrafx-16 HuCards.
    PcEngine,
    /// PC Engine CD-ROM² discs; the player's System Card (`syscard3.pce`) in `BIOS/`.
    PcEngineCd,
    /// Neo Geo Pocket and Pocket Color.
    NeoGeoPocket,
    /// WonderSwan and WonderSwan Color.
    WonderSwan,
    /// Famicom Disk System; the player's `disksys.rom` in `BIOS/`.
    DiskSystem,
    /// Sega SG-1000. Its folder is `SG1000`.
    Sg1000,
    /// Sony PlayStation: discs as `.chd`, `.cue` with its tracks, `.pbp`, or an `.m3u` listing a
    /// game's discs. The BIOS is the player's own and optional (the core has its own, HLE).
    PlayStation,
}
/// A core nibcase registers with DSlot: `/usr/lib/libretro/<name>_libretro.so`, described as
/// `nibcase-<name>`, playing these systems (the first is the one it is inferred for).
#[derive(Clone, Copy)]
pub struct CoreSpec {
    pub name: &'static str,
    pub systems: &'static [System],
}
pub const CORES: &[CoreSpec] = &[
    CoreSpec {
        name: "snes9x",
        systems: &[System::Snes],
    },
    CoreSpec {
        name: "nestopia",
        systems: &[System::Nes, System::DiskSystem],
    },
    CoreSpec {
        name: "blastem",
        systems: &[System::MegaDrive],
    },
    CoreSpec {
        name: "gearsystem",
        systems: &[System::GameGear, System::MasterSystem, System::Sg1000],
    },
    CoreSpec {
        name: "handy",
        systems: &[System::Lynx],
    },
    CoreSpec {
        name: "gw",
        systems: &[System::GameAndWatch],
    },
    CoreSpec {
        name: "mednafen_pce_fast",
        systems: &[System::PcEngine, System::PcEngineCd],
    },
    CoreSpec {
        name: "mednafen_ngp",
        systems: &[System::NeoGeoPocket],
    },
    CoreSpec {
        name: "mednafen_wswan",
        systems: &[System::WonderSwan],
    },
    CoreSpec {
        name: "pcsx_rearmed",
        systems: &[System::PlayStation],
    },
];
impl System {
    pub const ALL: [Self; 14] = [
        Self::Snes,
        Self::Nes,
        Self::MegaDrive,
        Self::GameGear,
        Self::Lynx,
        Self::GameAndWatch,
        Self::MasterSystem,
        Self::PcEngine,
        Self::PcEngineCd,
        Self::NeoGeoPocket,
        Self::WonderSwan,
        Self::DiskSystem,
        Self::Sg1000,
        Self::PlayStation,
    ];
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.id() == id)
    }
    /// Its folder under `Roms/` and `Saves/`, as the stock OS names them.
    pub fn id(self) -> &'static str {
        match self {
            Self::Snes => "SFC",
            Self::Nes => "FC",
            Self::MegaDrive => "MD",
            Self::GameGear => "GG",
            Self::Lynx => "LYNX",
            Self::GameAndWatch => "GW",
            Self::MasterSystem => "SMS",
            Self::PcEngine => "PCE",
            Self::PcEngineCd => "PCECD",
            Self::NeoGeoPocket => "NGP",
            Self::WonderSwan => "WS",
            Self::DiskSystem => "FDS",
            Self::Sg1000 => "SG1000",
            Self::PlayStation => "PS",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Snes => "SNES",
            Self::Nes => "NES",
            Self::MegaDrive => "Mega Drive",
            Self::GameGear => "Game Gear",
            Self::Lynx => "Lynx",
            Self::GameAndWatch => "Game & Watch",
            Self::MasterSystem => "Master System",
            Self::PcEngine => "PC Engine",
            Self::PcEngineCd => "PC Engine CD",
            Self::NeoGeoPocket => "Neo Geo Pocket",
            Self::WonderSwan => "WonderSwan",
            Self::DiskSystem => "Disk System",
            Self::Sg1000 => "SG-1000",
            Self::PlayStation => "PlayStation",
        }
    }
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Snes => &["sfc", "smc"],
            Self::Nes => &["nes", "unf", "unif"],
            Self::MegaDrive => &["md", "gen", "bin", "smd"],
            Self::GameGear => &["gg"],
            Self::Lynx => &["lnx"],
            Self::GameAndWatch => &["mgw"],
            Self::MasterSystem => &["sms"],
            Self::PcEngine => &["pce"],
            // A disc's tracks are listed in its .cue; a .chd holds them all.
            Self::PcEngineCd => &["cue", "chd"],
            Self::NeoGeoPocket => &["ngp", "ngc"],
            Self::WonderSwan => &["ws", "wsc"],
            Self::DiskSystem => &["fds"],
            Self::Sg1000 => &["sg"],
            // An .m3u lists a game's discs; .cue lists a disc's tracks (the .bin files).
            Self::PlayStation => &["m3u", "chd", "cue", "pbp"],
        }
    }
    /// The core that plays it by default.
    pub fn default_core(self) -> &'static str {
        CORES
            .iter()
            .find(|c| c.systems.contains(&self))
            .map(|c| c.name)
            .expect("every system has a core")
    }
    /// Whether its games say something about themselves that sets a core option
    /// ([`rom_defaults`]): only these are read before loading.
    pub fn has_rom_defaults(self) -> bool {
        self == Self::WonderSwan
    }
    /// BIOS files it plays better with but doesn't need (the core has its own), any one of them
    /// in `BIOS/`: the PlayStation's, as libretro's PCSX ReARMed page lists them.
    pub fn optional_bios(self) -> &'static [&'static str] {
        match self {
            Self::PlayStation => &[
                "scph5501.bin",
                "scph1001.bin",
                "scph7001.bin",
                "scph101.bin",
                "PSXONPSP660.bin",
            ],
            _ => &[],
        }
    }
    /// Whether its games are discs (never zipped, often several files).
    pub fn discs(self) -> bool {
        matches!(self, Self::PcEngineCd | Self::PlayStation)
    }
    /// The BIOS file it needs in `BIOS/`, if any.
    pub fn bios(self) -> Option<&'static str> {
        match self {
            Self::Lynx => Some("lynxboot.img"),
            Self::PcEngineCd => Some("syscard3.pce"),
            Self::DiskSystem => Some("disksys.rom"),
            _ => None,
        }
    }
    pub fn takes(self, path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            // Discs aren't played from inside a zip.
            (e.eq_ignore_ascii_case("zip") && !self.discs())
                || self.extensions().iter().any(|x| e.eq_ignore_ascii_case(x))
        })
    }
    pub fn from_rom(path: &Path) -> Option<Self> {
        if let Some(system) = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|p| p.to_str())
            .and_then(Self::from_id)
        {
            return Some(system);
        }
        let inner = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
        {
            Path::new(path.file_stem()?).to_owned()
        } else {
            path.to_owned()
        };
        let ext = inner.extension()?.to_str()?;
        Self::ALL
            .into_iter()
            .find(|s| s.extensions().iter().any(|e| ext.eq_ignore_ascii_case(e)))
    }
    pub fn from_core(path: &Path) -> Option<Self> {
        Self::of_core(path).first().copied()
    }
    /// The systems the core at `path` plays, by its file name; none for a core nibcase doesn't
    /// register.
    pub fn of_core(path: &Path) -> &'static [System] {
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            return &[];
        };
        CORES
            .iter()
            .find(|c| stem == format!("{}_libretro", c.name))
            .map_or(&[], |c| c.systems)
    }
}
/// Core option defaults a game's own bytes call for, as (key, value), below anything the player
/// set. A WonderSwan game says in its footer whether it is played holding the console upright:
/// bit 0 of the byte 4 from the end (WSdev wiki, "ROM header"). Upright games are shown upright,
/// filling more of the panel; the others across.
pub fn rom_defaults(system: System, rom: &[u8]) -> Vec<(&'static str, &'static str)> {
    match system {
        System::WonderSwan if rom.len() >= 16 => {
            let upright = rom[rom.len() - 4] & 1 != 0;
            vec![(
                "wswan_rotate_display",
                if upright { "portrait" } else { "landscape" },
            )]
        }
        _ => Vec::new(),
    }
}

/// The player starts NES with raw pixels, so integer scaling retains the cartridge's
/// raster and does not turn a 256-pixel frame into a 602-pixel composite-filter image.
/// The descriptor and in-game defaults must agree; users can still select the filter.
pub fn option_default<'a>(id: &str, option: &'a crate::OptionDef) -> &'a str {
    if id == "nibcase-nestopia"
        && option.key == "nestopia_blargg_ntsc_filter"
        && option.values.iter().any(|v| v.0 == "disabled")
    {
        "disabled"
    } else {
        &option.default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_nes_default_keeps_declared_choices() {
        let mut option = crate::OptionDef {
            key: "nestopia_blargg_ntsc_filter".into(),
            label: "Filter".into(),
            info: String::new(),
            category: String::new(),
            values: vec![
                ("disabled".into(), "Disabled".into()),
                ("composite".into(), "Composite".into()),
            ],
            default: "composite".into(),
            visible: true,
        };
        assert_eq!(option_default("nibcase-nestopia", &option), "disabled");
        assert_eq!(option_default("nibcase-other", &option), "composite");
        option.values.remove(0);
        assert_eq!(option_default("nibcase-nestopia", &option), "composite");
    }
    #[test]
    fn new_cartridge_systems() {
        for (system, names) in [
            (
                System::MegaDrive,
                vec![
                    "Coast.MD",
                    "Coast.gen.zip",
                    "/Roms/MD/Coast.zip",
                    "Coast.smd",
                ],
            ),
            (
                System::GameGear,
                vec!["Coast.GG", "Coast.gg.zip", "/Roms/GG/Coast.zip"],
            ),
            (
                System::Lynx,
                vec!["Coast.LNX", "Coast.lnx.zip", "/Roms/LYNX/Coast.zip"],
            ),
            (
                System::GameAndWatch,
                vec!["Parachute.mgw", "Parachute.MGW", "/Roms/GW/Parachute.zip"],
            ),
            (
                System::MasterSystem,
                vec!["Coast.sms", "Coast.sms.zip", "/Roms/SMS/Coast.zip"],
            ),
            (
                System::PcEngine,
                vec!["Coast.pce", "Coast.PCE.zip", "/Roms/PCE/Coast.zip"],
            ),
            (
                System::PcEngineCd,
                vec!["Coast.cue", "Coast.chd", "/Roms/PCECD/Coast.chd"],
            ),
            (
                System::NeoGeoPocket,
                vec!["Coast.ngp", "Coast.ngc", "/Roms/NGP/Coast.zip"],
            ),
            (
                System::WonderSwan,
                vec!["Coast.ws", "Coast.wsc.zip", "/Roms/WS/Coast.zip"],
            ),
            (System::DiskSystem, vec!["Coast.fds", "/Roms/FDS/Coast.zip"]),
            (System::Sg1000, vec!["Coast.sg", "/Roms/SG1000/Coast.zip"]),
            (
                System::PlayStation,
                // .chd and .cue are PC Engine CD's too: the folder decides.
                vec![
                    "Coast.m3u",
                    "/Roms/PS/Coast.chd",
                    "Coast.pbp",
                    "/Roms/PS/Coast (Disc 1).cue",
                ],
            ),
        ] {
            assert_eq!(System::from_id(system.id()), Some(system));
            for name in names {
                assert_eq!(System::from_rom(Path::new(name)), Some(system));
            }
            let core = format!("{}_libretro.so", system.default_core());
            assert!(System::of_core(Path::new(&core)).contains(&system));
        }
    }
    #[test]
    fn wonderswan_games_say_which_way_up() {
        let mut rom = vec![0u8; 1024];
        assert_eq!(
            rom_defaults(System::WonderSwan, &rom),
            [("wswan_rotate_display", "landscape")]
        );
        rom[1020] = 0x05;
        assert_eq!(
            rom_defaults(System::WonderSwan, &rom),
            [("wswan_rotate_display", "portrait")]
        );
        assert!(rom_defaults(System::WonderSwan, &[1; 8]).is_empty());
        assert!(rom_defaults(System::Snes, &rom).is_empty());
    }
    #[test]
    fn one_core_plays_several_systems_and_bios_is_named() {
        let gearsystem = Path::new("/usr/lib/libretro/gearsystem_libretro.so");
        assert_eq!(System::from_core(gearsystem), Some(System::GameGear));
        assert_eq!(
            System::of_core(gearsystem),
            [System::GameGear, System::MasterSystem, System::Sg1000]
        );
        assert_eq!(System::DiskSystem.default_core(), "nestopia");
        assert_eq!(System::PcEngineCd.default_core(), "mednafen_pce_fast");
        assert_eq!(System::DiskSystem.bios(), Some("disksys.rom"));
        assert_eq!(System::PcEngineCd.bios(), Some("syscard3.pce"));
        assert_eq!(System::PcEngine.bios(), None);
        assert!(System::of_core(Path::new("other_libretro.so")).is_empty());
        assert!(!System::PcEngineCd.takes(Path::new("Coast.zip")));
        assert!(System::PcEngine.takes(Path::new("Coast.zip")));
    }
    #[test]
    fn inference_and_cartridge_scope() {
        for name in ["game.NES", "game.nes.zip", "game.UNIF", "/Roms/FC/game.zip"] {
            assert_eq!(System::from_rom(Path::new(name)), Some(System::Nes));
        }
        assert_eq!(
            System::from_rom(Path::new("game.smc.zip")),
            Some(System::Snes)
        );
        assert_eq!(
            System::from_core(Path::new("/usr/lib/libretro/nestopia_libretro.so")),
            Some(System::Nes)
        );
        assert!(!System::Nes.takes(Path::new("disk.fds")));
        assert!(!System::Nes.takes(Path::new("music.nsf")));
    }
}
