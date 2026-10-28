//! The shared nibkit menu: Play, state cards, game/console settings, and live device controls.
use crate::connections::{Connections, HINT};
use crate::{
    saves::{Saves, State},
    settings::{self, Preferences},
};
use nibcase_libretro::Core;
use nibkit::menu::{self, Effect, Key, Row, Value};
use nibkit::screens::Screen;
use nibkit::settings::Def;

#[derive(Clone)]
enum Item {
    Resume,
    Save,
    Load,
    Fast,
    /// The disc in the drive, for a game of several.
    Disc,
    Reset,
    Quit,
    Setting(Def),
    Volume,
    Brightness(Screen),
    Clock,
    Protect,
    Heading,
    /// A line of help: shown, does nothing.
    Info,
    /// A row of the Connections page.
    Connection,
}
/// The Connections page's place among the tabs, after System.
const CONNECTIONS: usize = 5;
pub enum Command {
    None,
    Resume,
    Quit,
    Reset,
    Save(usize),
    Load(usize),
    Fast,
    /// Put disc `n` (from 0) in the drive.
    Disc(u32),
}
pub struct Menu {
    pub page: usize,
    pub rows: menu::Menu,
    pub cards: menu::Shelf,
    pub states: Vec<Option<State>>,
    pub notice: String,
    items: Vec<Item>,
    armed: Option<usize>,
    pub last_slot: usize,
    /// For a game of several discs: the disc in the drive, and each disc's name.
    pub discs: Option<(u32, Vec<String>)>,
    /// The Connections page while the menu is open (nibOS), dropped when it closes.
    connections: Option<Connections>,
    brightness_dirty: bool,
    /// The States page was just opened: its shelf starts on the first numbered slot.
    opening_states: bool,
}
impl Menu {
    pub fn new(saves: &Saves) -> Self {
        let states = (0..=6)
            .map(|slot| State::read(&saves.path(slot)).ok())
            .collect();
        Self {
            page: 0,
            rows: menu::Menu::default(),
            cards: menu::Shelf::default(),
            states,
            notice: String::new(),
            items: Vec::new(),
            armed: None,
            last_slot: 1,
            brightness_dirty: false,
            opening_states: false,
            discs: None,
            connections: None,
        }
    }
    fn tabs(&self, system: &str) -> Vec<(String, bool)> {
        let mut tabs = vec!["Play", "States", "Game", system, "System"];
        if self.connections.is_some() {
            tabs.push("Connections");
        }
        tabs.iter()
            .enumerate()
            .map(|(i, s)| (s.to_string(), i == self.page))
            .collect()
    }
    /// Keeps the Connections page while the menu is `open`: started when it opens, dropped (its
    /// worker stops) when it closes.
    pub fn keep_connections(&mut self, open: bool) {
        if open && self.connections.is_none() {
            self.connections = Connections::start();
        } else if !open && self.connections.is_some() {
            self.connections = None;
            if self.page == CONNECTIONS {
                self.page = 0;
            }
        }
    }
    pub fn rebuild(&mut self, prefs: &Preferences, core: &Core, fast: bool) {
        self.items.clear();
        let mut rows = Vec::new();
        let mut add = |label: String, value: Value, item: Item| {
            rows.push(if matches!(item, Item::Heading) {
                Row::heading(label)
            } else {
                Row::new(label, value)
            });
            self.items.push(item);
        };
        match self.page {
            0 => {
                add("Resume".into(), Value::None, Item::Resume);
                add(
                    "Save state".into(),
                    Value::More(format!("Slot {}", self.last_slot)),
                    Item::Save,
                );
                add(
                    "Load state".into(),
                    Value::More(format!("Slot {}", self.last_slot)),
                    Item::Load,
                );
                if let Some((at, names)) = &self.discs {
                    let name = names.get(*at as usize).cloned().unwrap_or_default();
                    add(
                        "Disc".into(),
                        Value::More(format!("{name} ({} of {})", at + 1, names.len())),
                        Item::Disc,
                    );
                }
                add(
                    "Fast-forward".into(),
                    Value::Text(if fast { "On (2x)" } else { "Off" }.into()),
                    Item::Fast,
                );
                add(
                    "Start over".into(),
                    if self.armed == Some(99) {
                        Value::Prompt {
                            text: "A again to restart".into(),
                            warning: true,
                        }
                    } else {
                        Value::None
                    },
                    Item::Reset,
                );
                add("Save and quit".into(), Value::None, Item::Quit);
                if prefs.system == "GW" {
                    add("Controls".into(), Value::None, Item::Heading);
                    for (what, does) in crate::unit::CONTROLS {
                        add((*what).into(), Value::Text((*does).into()), Item::Info);
                    }
                }
            }
            1 => {}
            2 | 3 => {
                let game = self.page == 2;
                add("Display and play".into(), Value::None, Item::Heading);
                for d in settings::frontend(&prefs.system) {
                    add(d.label.clone(), prefs.show(&d, game), Item::Setting(d));
                }
                let mut options: Vec<_> = core.options().filter(|d| d.visible).collect();
                options.sort_by_key(|d| (&d.category, &d.key));
                let mut category = String::new();
                for option in options {
                    let heading = if option.category.is_empty() {
                        core.info().name.clone()
                    } else {
                        option.category.clone()
                    };
                    if category != heading {
                        category = heading;
                        add(category.clone(), Value::None, Item::Heading);
                    }
                    let d = settings::core_def(&prefs.id, option);
                    add(d.label.clone(), prefs.show(&d, game), Item::Setting(d));
                }
            }
            CONNECTIONS if self.connections.is_some() => {
                let c = self.connections.as_mut().unwrap();
                c.refresh();
                for row in c.rows() {
                    rows.push(row);
                    self.items.push(Item::Connection);
                }
            }
            _ => {
                if let Some(v) = nibkit::device::volume() {
                    add("Volume".into(), Value::Slider(v), Item::Volume);
                }
                for (screen, name) in [
                    (Screen::Top, "Top screen"),
                    (Screen::Bottom, "Bottom screen"),
                ] {
                    if let Some(v) = nibkit::backlight::get(screen) {
                        add(name.into(), Value::Slider(v), Item::Brightness(screen));
                    }
                }
                add(
                    "12-hour clock".into(),
                    Value::Text(if nibkit::clock::hour12() { "On" } else { "Off" }.into()),
                    Item::Clock,
                );
                if let Some(on) = nibkit::battery::protect() {
                    add(
                        "Battery protect".into(),
                        Value::Text(if on { "On" } else { "Off" }.into()),
                        Item::Protect,
                    );
                }
                add(
                    format!("Core: {} {}", core.info().name, core.info().version),
                    Value::None,
                    Item::Heading,
                );
            }
        }
        let title = match self.page {
            0 => nibkit::names::display_name(&prefs.name),
            1 => "Save states".into(),
            2 => format!("{} · this game", nibkit::names::display_name(&prefs.name)),
            3 => format!(
                "{} defaults",
                nibcase_libretro::systems::System::from_id(&prefs.system)
                    .unwrap()
                    .label()
            ),
            CONNECTIONS => self
                .connections
                .as_ref()
                .map_or_else(|| "System".into(), Connections::title),
            _ => "System".into(),
        };
        self.rows = menu::Menu {
            tabs: self.tabs(
                nibcase_libretro::systems::System::from_id(&prefs.system)
                    .unwrap()
                    .label(),
            ),
            title,
            rows,
            row: self.rows.row,
            scroll: self.rows.scroll,
            hint: if !self.notice.is_empty() {
                self.notice.clone()
            } else if self.page == CONNECTIONS && self.connections.is_some() {
                HINT.into()
            } else {
                "A  choose   ·   B  resume   ·   Y  inherit   ·   L / R  page".into()
            },
        };
        self.rows.settle();
        let tabs = self.tabs(
            nibcase_libretro::systems::System::from_id(&prefs.system)
                .unwrap()
                .label(),
        );
        let at = if std::mem::take(&mut self.opening_states) {
            None
        } else {
            Some(self.cards.at)
        };
        self.cards = shelf(&self.states, tabs, at, &self.notice);
    }
    pub fn key(
        &mut self,
        key: Key,
        prefs: &mut Preferences,
        core: &mut Core,
    ) -> Result<Command, String> {
        let effect = if self.page == 1 {
            self.cards.key(key)
        } else {
            self.rows.key(key)
        };
        self.effect(effect, prefs, core)
    }
    pub fn touch(
        &mut self,
        x: f32,
        y: f32,
        prefs: &mut Preferences,
        core: &mut Core,
    ) -> Result<Command, String> {
        let effect = if self.page == 1 {
            self.cards.touch(x, y)
        } else {
            self.rows.touch(x, y)
        };
        self.effect(effect, prefs, core)
    }
    fn effect(
        &mut self,
        effect: Option<Effect>,
        prefs: &mut Preferences,
        core: &mut Core,
    ) -> Result<Command, String> {
        let Some(effect) = effect else {
            return Ok(Command::None);
        };
        match effect {
            Effect::Back => return Ok(Command::Resume),
            Effect::Page(page) => {
                self.page = page;
                self.opening_states = page == 1;
                self.rows.row = 0;
                self.rows.scroll = 0;
                self.notice.clear();
                self.armed = None;
            }
            Effect::Card { at, key } => {
                if key == Key::A {
                    if self.states[at].is_some() {
                        return Ok(Command::Load(at));
                    }
                    self.notice = "This slot is empty".into();
                } else if key == Key::X && at != 0 {
                    if self.states[at].is_none() || self.armed == Some(at) {
                        self.armed = None;
                        self.last_slot = at;
                        return Ok(Command::Save(at));
                    }
                    self.armed = Some(at);
                    self.notice = "X again to replace this state".into();
                }
            }
            Effect::Choose(row) | Effect::Change { row, .. } | Effect::Use(row) => {
                let Some(item) = self.items.get(row).cloned() else {
                    return Ok(Command::None);
                };
                // A radio or a device: A or Left / Right toggles it (Y has nothing to undo).
                if matches!(item, Item::Connection) {
                    if !matches!(effect, Effect::Use(_)) {
                        if let Some(c) = self.connections.as_mut() {
                            c.act(row);
                        }
                    }
                    return Ok(Command::None);
                }
                let step = if let Effect::Change { step, .. } = effect {
                    step
                } else {
                    1
                };
                match item {
                    Item::Resume => return Ok(Command::Resume),
                    Item::Quit => return Ok(Command::Quit),
                    Item::Save => return Ok(Command::Save(self.last_slot)),
                    Item::Load => return Ok(Command::Load(self.last_slot)),
                    Item::Fast => return Ok(Command::Fast),
                    // A, or Left / Right: the next or the previous disc.
                    Item::Disc => {
                        if let Some((at, names)) = &self.discs {
                            let n = names.len() as i64;
                            let next = (i64::from(*at) + i64::from(step)).rem_euclid(n);
                            return Ok(Command::Disc(next as u32));
                        }
                    }
                    Item::Reset => {
                        if self.armed == Some(99) {
                            self.armed = None;
                            return Ok(Command::Reset);
                        }
                        self.armed = Some(99);
                    }
                    Item::Setting(def) => {
                        if matches!(effect, Effect::Use(_)) {
                            prefs.reset(&def, self.page == 2)?;
                        } else {
                            prefs.change(&def, self.page == 2, step)?;
                        }
                        prefs.apply(core, false)?;
                        if def.label.to_ascii_lowercase().contains("restart") {
                            self.notice = "Start over to apply this core option".into();
                        }
                    }
                    Item::Volume => {
                        let v = nibkit::device::volume().unwrap_or(50);
                        nibkit::device::set_volume((i32::from(v) + step * 5).clamp(0, 100) as u8);
                    }
                    Item::Brightness(screen) => {
                        let v = nibkit::backlight::get(screen).unwrap_or(50);
                        nibkit::backlight::set(
                            screen,
                            (i32::from(v) + step * 5).clamp(0, 100) as u8,
                        );
                        self.brightness_dirty = true;
                    }
                    Item::Clock => {
                        nibkit::clock::set_hour12(!nibkit::clock::hour12())
                            .map_err(|e| e.to_string())?;
                    }
                    Item::Protect => {
                        nibkit::battery::set_protect(!nibkit::battery::protect().unwrap_or(false))
                            .map_err(|e| e.to_string())?;
                    }
                    Item::Heading | Item::Info | Item::Connection => {}
                }
            }
        }
        Ok(Command::None)
    }
    pub fn settled(&mut self) {
        if self.brightness_dirty {
            nibkit::backlight::save();
            self.brightness_dirty = false;
        }
    }
    pub fn picture(&self, index: usize) -> Option<(&[u8], usize, usize)> {
        let frame = &self.states.get(index)?.as_ref()?.picture;
        (frame.width > 0).then_some((
            frame.rgba.as_slice(),
            frame.width as usize,
            frame.height as usize,
        ))
    }
}
/// The States page: the resume point first, as its own tile labelled with when it was written,
/// then the numbered slots. `at` is the card lit before, or `None` when the page has just opened
/// (it then starts on slot 1; Left reaches the resume point). Card `n` is state `n`.
fn shelf(
    states: &[Option<State>],
    tabs: Vec<(String, bool)>,
    at: Option<usize>,
    notice: &str,
) -> menu::Shelf {
    let cards = states
        .iter()
        .enumerate()
        .map(|(slot, state)| {
            let picture = state.as_ref().map(|_| slot);
            let written = state.as_ref().map(|s| when(s.metadata.created));
            if slot == 0 {
                menu::Card::resume(picture, written.unwrap_or_else(|| "Not yet".into()))
            } else {
                menu::Card::slot(
                    picture,
                    format!("Slot {slot} · {}", written.as_deref().unwrap_or("Empty")),
                )
            }
        })
        .collect();
    let mut shelf = menu::Shelf {
        tabs,
        cards,
        at: 0,
        hint: String::new(),
    };
    shelf.at = match at {
        Some(at) => at.min(shelf.cards.len().saturating_sub(1)),
        None => shelf.first_slot(),
    };
    // The resume point is written by the game itself: it can be loaded, never saved over.
    shelf.hint = if !notice.is_empty() {
        notice.into()
    } else if shelf.cards.get(shelf.at).is_some_and(|c| c.resume) {
        "A  load   ·   B  resume".into()
    } else {
        "A  load   ·   X  save (again to replace)   ·   B  resume".into()
    };
    shelf
}

/// When a state was written, as the menus show it: "Oct 20 9:41 PM", on the player's clock.
fn when(secs: u64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let t = local(secs as libc::time_t);
    format!(
        "{} {} {}",
        MONTHS[t.tm_mon.clamp(0, 11) as usize],
        t.tm_mday,
        nibkit::clock::format(t.tm_hour as u32, t.tm_min as u32, nibkit::clock::hour12())
    )
}

fn local(secs: libc::time_t) -> libc::tm {
    // SAFETY: a zeroed tm is valid writable output for localtime_r; it has no Rust invariants.
    let mut t: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are valid; localtime_r does not retain them.
    unsafe { libc::localtime_r(&secs, &mut t) };
    t
}

pub fn status() -> menu::Status {
    let mut now = 0;
    // SAFETY: time writes one time_t to valid local storage.
    unsafe { libc::time(&mut now) };
    let t = local(now);
    menu::Status {
        clock: nibkit::clock::format(t.tm_hour as u32, t.tm_min as u32, nibkit::clock::hour12()),
        battery: nibkit::battery::read()
            .map(|b| (b.percent, b.state == nibkit::battery::State::Charging)),
        ..menu::Status::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saves::Metadata;
    use nibcase_libretro::Frame;

    fn state() -> Option<State> {
        Some(State {
            metadata: Metadata {
                core: "Snes9x".into(),
                version: "1".into(),
                game_hash: 1,
                created: 0,
                play_seconds: 0,
                state_hash: 0,
            },
            picture: Frame {
                width: 0,
                height: 0,
                rgba: Vec::new(),
            },
            data: Vec::new(),
            progress: None,
        })
    }

    #[test]
    fn the_resume_point_is_its_own_first_tile_and_the_page_opens_on_slot_1() {
        let mut states = vec![None; 7];
        states[0] = state();
        states[2] = state();
        let shelf = shelf(&states, Vec::new(), None, "");
        assert!(shelf.cards[0].resume);
        assert!(shelf.cards[1..].iter().all(|c| !c.resume));
        assert_eq!(shelf.cards[0].label, when(0));
        assert_eq!(shelf.cards[1].label, "Slot 1 · Empty");
        assert_eq!(shelf.cards[2].label, format!("Slot 2 · {}", when(0)));
        assert_eq!(shelf.at, 1);
        assert!(shelf.hint.contains("X  save"));
    }

    #[test]
    fn on_the_resume_tile_x_isnt_offered() {
        let shelf = shelf(&vec![None; 7], Vec::new(), Some(0), "");
        assert_eq!(shelf.at, 0);
        assert_eq!(shelf.cards[0].label, "Not yet");
        assert!(!shelf.hint.contains('X'));
        assert_eq!(super::shelf(&vec![None; 7], Vec::new(), Some(9), "").at, 6);
    }
}
