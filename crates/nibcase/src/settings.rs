//! Layered frontend settings and the core options shared with DSlot's installed description.
use nibcase_libretro::{Core, OptionDef};
use nibkit::settings::{Def, Kind, Levels, Scope, Settings, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn stem(path: &Path) -> String {
    let mut name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        if let Some((base, ext)) = name.rsplit_once('.') {
            if nibcase_libretro::systems::System::ALL
                .into_iter()
                .any(|s| s.extensions().iter().any(|e| ext.eq_ignore_ascii_case(e)))
            {
                name = base.to_owned();
            }
        }
    }
    name
}
pub fn core_id(path: &Path) -> String {
    let name = path.file_stem().unwrap_or_default().to_string_lossy();
    format!(
        "nibcase-{}",
        name.strip_suffix("_libretro").unwrap_or(&name)
    )
}
fn choice(key: &str, label: &str, values: &[(&str, &str)], default: &str) -> Def {
    Def {
        key: key.into(),
        label: label.into(),
        kind: Kind::choice(values),
        levels: Levels::CONSOLE_AND_GAME,
        default: Some(Value::from(default)),
    }
}
/// nibcase's own settings for a game of `system`. A Game & Watch unit is a photograph, not a
/// raster of pixels, so it is fitted to the panel by default rather than scaled by whole steps,
/// and it can be shown whole or as its LCDs alone. A PlayStation game changes resolution as it
/// goes (320 x 240, 512 x 240, 640 x 480) on a 4:3 television, so it is fitted at the TV's
/// shape; its pad is the player's choice.
pub fn frontend(system: &str) -> Vec<Def> {
    let mut defs = vec![
        choice(
            "scale",
            "Scale",
            &[("integer", "Integer"), ("fit", "Fit")],
            if matches!(system, "GW" | "PS") {
                "fit"
            } else {
                "integer"
            },
        ),
        choice(
            "aspect",
            "Aspect",
            &[("square", "Square pixels"), ("tv", "Console TV shape")],
            if system == "PS" { "tv" } else { "square" },
        ),
        choice(
            "timing",
            "Timing",
            &[("smooth", "Smooth"), ("exact", "Exact speed")],
            "smooth",
        ),
        choice(
            "bottom",
            "Bottom screen in play",
            &[("off", "Off"), ("on", "On")],
            "off",
        ),
        Def {
            key: "resume".into(),
            label: "Resume where I left off".into(),
            kind: Kind::Toggle,
            levels: Levels::CONSOLE_AND_GAME,
            default: Some(true.into()),
        },
        Def {
            key: "stick".into(),
            label: "Stick as D-pad".into(),
            kind: Kind::Toggle,
            levels: Levels::CONSOLE_AND_GAME,
            default: Some(true.into()),
        },
    ];
    if system == "PS" {
        defs.insert(
            0,
            choice(
                "pad",
                "Controller",
                &[
                    ("digital", "Digital pad"),
                    ("dualshock", "DualShock (analog stick)"),
                ],
                "digital",
            ),
        );
    }
    if system == "GW" {
        // Game & Watch: the whole unit as photographed, or its LCDs alone (unit.rs).
        defs.insert(
            0,
            choice(
                "show",
                "Show",
                &[("unit", "Whole unit"), ("screen", "Screen only")],
                "unit",
            ),
        );
        defs.insert(
            1,
            choice(
                "lcd",
                "LCD in Screen only",
                &[("bright", "Brighter"), ("photo", "As photographed")],
                "bright",
            ),
        );
    }
    defs
}
pub fn core_def(core: &str, option: &OptionDef) -> Def {
    Def {
        key: format!("{core}.{}", option.key),
        label: option.label.clone(),
        kind: Kind::Choice(
            option
                .values
                .iter()
                .map(|(v, l)| (Value::from(v.clone()), l.clone()))
                .collect(),
        ),
        levels: Levels::CONSOLE_AND_GAME,
        default: Some(nibcase_libretro::systems::option_default(core, option).into()),
    }
}
pub struct Preferences {
    pub id: String,
    pub system: String,
    pub name: String,
    own: Settings,
    shared: Option<Settings>,
    declared: BTreeSet<String>,
    cli: BTreeMap<String, String>,
    /// Core option defaults this game's own bytes call for (`systems::rom_defaults`): below
    /// every setting, above the core's default.
    pub rom_defaults: BTreeMap<String, String>,
}
impl Preferences {
    pub fn new(core: &Path, system: &str, name: &str, cli: BTreeMap<String, String>) -> Self {
        let id = core_id(core);
        let own = Settings::load(
            nibkit::paths::AppDirs::new("nibcase")
                .config
                .join("settings.toml"),
        );
        #[cfg(feature = "dslot")]
        let (shared, declared) = {
            let descriptions = nibkit::dslot::Emulators::load(&nibkit::dslot::description_dirs());
            let declared = descriptions
                .get(&id)
                .map(|e| e.options_for(system).map(|o| o.def.key.clone()).collect())
                .unwrap_or_default();
            (nibkit::dslot::settings_file().map(Settings::load), declared)
        };
        #[cfg(not(feature = "dslot"))]
        let (shared, declared) = (None, BTreeSet::new());
        Self {
            id,
            system: system.into(),
            name: name.into(),
            own,
            shared,
            declared,
            cli,
            rom_defaults: BTreeMap::new(),
        }
    }
    pub fn scope(&self, game: bool) -> Scope {
        if game {
            Scope::Game(Scope::game_key(&self.system, &self.name))
        } else {
            Scope::Console(self.system.clone())
        }
    }
    fn chain(&self, game: bool) -> Vec<Scope> {
        if game {
            vec![self.scope(true), self.scope(false)]
        } else {
            vec![self.scope(false)]
        }
    }
    fn store(&self, key: &str) -> &Settings {
        if self.declared.contains(key) {
            self.shared.as_ref().unwrap_or(&self.own)
        } else {
            &self.own
        }
    }
    fn store_mut(&mut self, key: &str) -> &mut Settings {
        if self.declared.contains(key) {
            self.shared.as_mut().unwrap_or(&mut self.own)
        } else {
            &mut self.own
        }
    }
    pub fn value(&self, def: &Def, game: bool) -> Value {
        let chain = self.chain(game);
        let store = self.store(&def.key);
        // Set at no level: what the game says about itself, before the option's default.
        let from_game = store
            .source(&chain, def)
            .is_none()
            .then(|| def.key.strip_prefix(&format!("{}.", self.id)))
            .flatten()
            .and_then(|key| self.rom_defaults.get(key))
            .map(|v| Value::from(v.as_str()))
            .filter(|v| def.accepts(v));
        from_game
            .or_else(|| store.value(&chain, def))
            .or_else(|| def.default.clone())
            .unwrap_or(Value::String(String::new()))
    }
    pub fn set_here(&self, def: &Def, game: bool) -> bool {
        self.store(&def.key)
            .get(&self.scope(game), &def.key)
            .is_some()
    }
    pub fn show(&self, def: &Def, game: bool) -> nibkit::menu::Value {
        let value = self.value(def, game);
        let shown = def.show(&value);
        if self.set_here(def, game) {
            nibkit::menu::Value::Text(shown)
        } else {
            nibkit::menu::Value::Inherited(format!(
                "Use {} ({shown})",
                if game {
                    self.system.as_str()
                } else {
                    "default"
                }
            ))
        }
    }
    pub fn change(&mut self, def: &Def, game: bool, step: i32) -> Result<(), String> {
        if let Some(key) = def.key.strip_prefix(&format!("{}.", self.id)) {
            self.cli.remove(key);
        }
        let scope = self.scope(game);
        let current = self.value(def, game);
        let next = if self.set_here(def, game) {
            def.step(Some(&current), step)
        } else {
            current
        };
        let store = self.store_mut(&def.key);
        store.set(scope, &def.key, next);
        store.save().map_err(|e| e.to_string())
    }
    pub fn reset(&mut self, def: &Def, game: bool) -> Result<(), String> {
        if let Some(key) = def.key.strip_prefix(&format!("{}.", self.id)) {
            self.cli.remove(key);
        }
        let scope = self.scope(game);
        let store = self.store_mut(&def.key);
        store.reset(&scope, &def.key);
        store.save().map_err(|e| e.to_string())
    }
    pub fn text(&self, key: &str) -> String {
        frontend(&self.system)
            .into_iter()
            .find(|d| d.key == key)
            .map(|d| self.value(&d, true).as_str().unwrap_or("").into())
            .unwrap_or_default()
    }
    pub fn enabled(&self, key: &str) -> bool {
        frontend(&self.system)
            .into_iter()
            .find(|d| d.key == key)
            .and_then(|d| self.value(&d, true).as_bool())
            .unwrap_or(false)
    }
    pub fn apply(&self, core: &mut Core, _initial: bool) -> Result<(), String> {
        let options: Vec<_> = core.options().cloned().collect();
        for option in options {
            let def = core_def(&self.id, &option);
            let value = self.value(&def, true);
            let value = self
                .cli
                .get(&option.key)
                .cloned()
                .unwrap_or_else(|| value.as_str().unwrap_or(&option.default).into());
            if core.value(&option.key) != Some(value.as_str()) {
                core.set_option(&option.key, &value)?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_match_dslot() {
        for name in [
            "Moon Bay.sfc",
            "Moon Bay.smc.zip",
            "Moon Bay.SFC.ZIP",
            "Moon Bay.md.zip",
            "Moon Bay.GG.ZIP",
            "Moon Bay.lnx.zip",
            "Moon Bay.NES.ZIP",
            "Moon Bay.unif.zip",
        ] {
            assert_eq!(stem(Path::new(name)), "Moon Bay");
        }
        assert_eq!(
            core_id(Path::new("/usr/lib/libretro/nestopia_libretro.so")),
            "nibcase-nestopia"
        );
        assert_eq!(
            core_id(Path::new("/usr/lib/libretro/snes9x_libretro.so")),
            "nibcase-snes9x"
        );
    }
}
