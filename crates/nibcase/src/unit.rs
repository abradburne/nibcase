//! Game & Watch units: how each is laid out over the two panels, and the buttons on its face.
//!
//! gw-libretro draws the whole unit, a photograph of it with its LCDs live inside, as one frame.
//! A unit that opens on a hinge (the Multi Screen and Panorama Screen ones) is cut at the hinge:
//! the lid on the top panel, the body on the bottom, both at one scale, meeting across the gap
//! between the panels as the real unit meets at its hinge. Every other unit is drawn whole on the
//! top panel, and the bottom panel carries touch buttons for its Game A / Game B / Time keys.
//!
//! The hinges were measured on the units' own frames (`nibcase-test --frame`, a
//! brightness profile across the middle of each); a unit is cut only when its name and frame size
//! are the measured ones, so a different conversion of a unit is drawn whole, never cut wrongly.
//!
//! "Screen only" (the `show` setting) leaves the unit out and shows its LCDs alone, as large as
//! they go. A unit with one LCD is zoomed by the core itself: its pointing hand (below) has a Zoom
//! key, at a place in its round that differs from unit to unit, measured for each one. A unit
//! with two LCDs has no Zoom; nibcase cuts the two out of the photograph, one on each panel.
//!
//! The core reaches a unit's keys through a pointing hand: Select moves it to the next key
//! (Game A, Game B, Time, then the unit's others), Start presses the key it points at and puts it
//! away. The touch buttons do the same, and on Nintendo units, whose keys come in that order, Game
//! A / Game B / Time are one tap each, starting from the hand put away.

use nibcase_libretro::ffi::{RETRO_DEVICE_ID_JOYPAD_SELECT, RETRO_DEVICE_ID_JOYPAD_START};

/// Where a unit opens, in its frame's pixels: the row of its hinge (a lid above a body) or the
/// column (a book opening left and right).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fold {
    Row(u32),
    Column(u32),
}

/// The units drawn in two parts, one on each panel: those that open on a hinge, and Penguin Land,
/// two consoles side by side. Name (the file's, without `.mgw`), frame size, where to cut.
const HINGED: &[(&str, (u32, u32), Fold)] = &[
    (
        "Donkey Kong (Nintendo, Multi Screen)",
        (606, 748),
        Fold::Row(375),
    ),
    (
        "Donkey Kong II (Nintendo, Multi Screen)",
        (605, 748),
        Fold::Row(376),
    ),
    (
        "Mario Bros. (Nintendo, Multi Screen)",
        (973, 532),
        Fold::Column(484),
    ),
    (
        "Lifeboat (Nintendo, Multi Screen)",
        (974, 532),
        Fold::Column(485),
    ),
    (
        "Donkey Kong Circus (Nintendo, Panorama Screen)",
        (498, 771),
        Fold::Row(400),
    ),
    (
        "Donkey Kong Jr. (Nintendo, Panorama Screen)",
        (499, 771),
        Fold::Row(407),
    ),
    (
        "Mario's Bombs Away (Nintendo, Panorama Screen)",
        (498, 770),
        Fold::Row(405),
    ),
    (
        "Mickey Mouse (Nintendo, Panorama Screen)",
        (493, 770),
        Fold::Row(402),
    ),
    (
        "Snoopy (Nintendo, Panorama Screen)",
        (498, 770),
        Fold::Row(404),
    ),
    (
        "Penguin Land (Bandai, LSI Game Double Play)",
        (1073, 580),
        Fold::Column(537),
    ),
];

/// The units with one LCD: name, frame size, how many Selects take the core's hand to Zoom.
/// Measured for each by pressing Select n times and Start until the frame shrank to the LCD.
const ZOOM: &[(&str, (u32, u32), u32)] = &[
    ("Armor Battle (Mattel Electronics)", (480, 759), 2),
    ("Banana (VTech, Time & Fun)", (562, 374), 4),
    ("Baseball (VTech, Explorer Time & Fun)", (612, 377), 5),
    ("Bomb Fight (VTech, Mini Time & Fun)", (540, 360), 4),
    ("Caccia al Ladro (VTech, Mini Time & Fun)", (542, 359), 4),
    ("Cessate il Fuoco (VTech, Mini Time & Fun)", (540, 360), 4),
    ("Chicky Woggy (VTech, Arcade Time & Fun)", (480, 664), 4),
    (
        "Chicky Woggy (VTech, Electronic Tini-Arcade)",
        (671, 747),
        4,
    ),
    ("Condor (VTech, Time & Fun)", (562, 373), 4),
    ("Crazy Chewy (VTech, Electronic Tini-Arcade)", (692, 747), 4),
    ("Defendo (VTech, Explorer Time & Fun)", (614, 378), 5),
    ("Donkey Angler (Gakken, LCD Card Game)", (611, 322), 3),
    ("Donkey Kong (Coleco)", (495, 745), 2),
    (
        "Donkey Kong Circus (Nintendo, Panorama Screen)",
        (498, 771),
        5,
    ),
    ("Donkey Kong Jr. (Coleco)", (480, 746), 5),
    ("Donkey Kong Jr. (Nintendo, Panorama Screen)", (499, 771), 5),
    ("Donkey Kong Jr. (Nintendo, Table Top)", (480, 745), 5),
    (
        "Dungeons & Dragons Computer Fantasy Game (Mattel Electronics, Arcade Action Series)",
        (527, 348),
        2,
    ),
    ("Egg (Nintendo, Wide Screen)", (653, 392), 1),
    ("Engine Room (VTech, Explorer Time & Fun)", (612, 377), 5),
    ("Escape (VTech, Time & Fun)", (564, 372), 4),
    ("Explorers of Space (Elektronika)", (654, 394), 6),
    ("Fowling (Elektronika)", (650, 393), 5),
    ("Frog Boaster (Elektronika)", (660, 397), 5),
    ("Galaxy II (Epoch)", (528, 747), 4),
    (
        "Grab Man (unknown manufacturer, Game & Time)",
        (480, 777),
        5,
    ),
    ("Hippo Teeth (VTech, Mini Time & Fun)", (542, 359), 4),
    ("Hippo Teeth (VTech, Sporty Time & Fun)", (480, 650), 4),
    ("Hot Line (VTech, Mini Time & Fun)", (542, 359), 4),
    ("Hot Line (VTech, Sporty Time & Fun)", (480, 656), 4),
    ("Las Vegas (Bandai, LCD Game Digital)", (546, 312), 4),
    (
        "Mario's Bombs Away (Nintendo, Panorama Screen)",
        (498, 770),
        5,
    ),
    ("Mickey Mouse (Nintendo, Panorama Screen)", (493, 770), 5),
    ("Mickey Mouse (Nintendo, Wide Screen)", (654, 396), 5),
    ("Monkey (VTech, Time & Fun)", (561, 374), 4),
    ("Monkey Jump (VTech, Arcade Time & Fun)", (480, 664), 4),
    ("Motor Cross (Queentex)", (480, 736), 5),
    ("Pac Man (Tomytronic)", (941, 740), 3),
    ("Pancake (VTech, Time & Fun)", (560, 372), 4),
    ("Parachute (Nintendo, Wide Screen)", (658, 395), 5),
    ("Pirate (VTech, Time & Fun)", (561, 374), 4),
    ("Roller Coaster (VTech, Explorer Time & Fun)", (609, 377), 5),
    ("Safari (VTech, Time & Fun)", (562, 372), 4),
    ("Sleep Walker (VTech, Time & Fun)", (558, 370), 4),
    ("Snoopy (Nintendo, Panorama Screen)", (498, 770), 5),
    ("Snoopy (Nintendo, Table Top)", (480, 743), 5),
    ("Snoopy Tennis (Nintendo, Wide Screen)", (654, 393), 5),
    ("Sub Chase (Mattel Electronics)", (480, 760), 2),
    ("Tennis Menace (VTech, Sporty Time & Fun)", (480, 656), 4),
    ("Tom & Jerry Popper (Gakken, LCD Card Game)", (643, 442), 3),
    ("Towering Rescue (Gakken, LCD Card Game)", (564, 445), 5),
    ("Tron (Tomytronic)", (766, 700), 2),
    ("Turtle Bridge (Nintendo, Wide Screen)", (658, 395), 5),
    (
        "Wild Man Jump (VTech, Electronic Tini-Arcade)",
        (697, 746),
        4,
    ),
];

/// The units with two LCDs (no Zoom): name, frame size, the LCDs' rectangles in the frame (the
/// first for the top panel), just inside their bezels.
const SCREENS: &[(&str, (u32, u32), Rect, Rect)] = &[
    (
        "Donkey Kong (Nintendo, Multi Screen)",
        (606, 748),
        (167, 103, 271, 175),
        (167, 462, 272, 173),
    ),
    (
        "Donkey Kong II (Nintendo, Multi Screen)",
        (605, 748),
        (165, 105, 270, 174),
        (165, 463, 270, 174),
    ),
    (
        "Mario Bros. (Nintendo, Multi Screen)",
        (973, 532),
        (102, 99, 291, 190),
        (573, 99, 285, 190),
    ),
    (
        "Lifeboat (Nintendo, Multi Screen)",
        (974, 532),
        (105, 101, 288, 187),
        (577, 101, 282, 187),
    ),
    (
        "Penguin Land (Bandai, LSI Game Double Play)",
        (1073, 580),
        (139, 120, 267, 193),
        (675, 124, 270, 193),
    ),
];

/// How many Selects reach the core's Zoom on the unit named `name` with frame `size`.
pub fn zoom(name: &str, size: (u32, u32)) -> Option<u32> {
    ZOOM.iter()
        .find(|(n, s, _)| *n == name && *s == size)
        .map(|&(_, _, k)| k)
}

/// The two LCDs of the unit named `name` with frame `size`, placed one on each panel: at one
/// scale, the largest at which both fit, each centred on its panel.
pub fn screens(name: &str, size: (u32, u32)) -> Option<(Part, Part)> {
    let &(_, _, a, b) = SCREENS
        .iter()
        .find(|(n, s, _, _)| *n == name && *s == size)?;
    let fits = |r: Rect| (PANEL.0 / r.2 as f32).min(PANEL.1 / r.3 as f32);
    let scale = fits(a).min(fits(b));
    let place = |r: Rect| {
        let (w, h) = (
            (r.2 as f32 * scale).round() as u32,
            (r.3 as f32 * scale).round() as u32,
        );
        Part {
            src: r,
            dst: ((1024 - w) / 2, (768 - h) / 2, w, h),
        }
    };
    Some((place(a), place(b)))
}

/// Where the unit named `name` opens, if it is one measured to and its frame is `size`.
pub fn fold(name: &str, size: (u32, u32)) -> Option<Fold> {
    HINGED
        .iter()
        .find(|(n, s, _)| *n == name && *s == size)
        .map(|&(_, _, f)| f)
}

/// A rectangle, `(x, y, w, h)`.
pub type Rect = (u32, u32, u32, u32);

/// One part of a cut unit: the frame's pixels it takes, and where they go on a 1024 x 768
/// panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    pub src: Rect,
    pub dst: Rect,
}

/// The panels' size.
const PANEL: (f32, f32) = (1024.0, 768.0);

/// A `w` x `h` frame cut at `fold`: the lid (or left page) for the top panel, the body (or right
/// page) for the bottom. Both parts take the largest scale at which each fits its panel, so the
/// unit keeps its proportions; each is centred across, the lid pushed down to the panel's bottom
/// edge and the body up to its top edge, so they meet across the gap between the panels.
pub fn split(w: u32, h: u32, fold: Fold) -> (Part, Part) {
    let (a, b): (Rect, Rect) = match fold {
        Fold::Row(y) => ((0, 0, w, y), (0, y, w, h - y)),
        Fold::Column(x) => ((0, 0, x, h), (x, 0, w - x, h)),
    };
    let fits = |r: Rect| (PANEL.0 / r.2 as f32).min(PANEL.1 / r.3 as f32);
    let scale = fits(a).min(fits(b));
    let place = |r: Rect, down: bool| {
        let (dw, dh) = (
            (r.2 as f32 * scale).round() as u32,
            (r.3 as f32 * scale).round() as u32,
        );
        let x = (1024 - dw) / 2;
        let y = if down { 768 - dh } else { 0 };
        Part {
            src: r,
            dst: (x, y, dw, dh),
        }
    };
    (place(a, true), place(b, false))
}

/// The touch buttons on the bottom panel under a unit drawn whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    GameA,
    GameB,
    Time,
    /// Select: the hand to the next key.
    Next,
    /// Start: press the key the hand points at.
    Press,
}

const SELECT: u16 = 1 << RETRO_DEVICE_ID_JOYPAD_SELECT;
const START: u16 = 1 << RETRO_DEVICE_ID_JOYPAD_START;
/// Frames a scripted button is held, then let go: long enough for the core to see each press.
const HOLD: usize = 3;

/// Where the core's hand is, as far as nibcase has seen the presses that move it, and the
/// presses still to send for a touch button.
#[derive(Debug, Default)]
pub struct Keys {
    /// Select presses since the hand was last put away (by Start); 0 when it is away.
    hand: u32,
    /// Whether the unit's keys come as Game A, Game B, Time (Nintendo's do).
    ordered: bool,
    /// The buttons to hold, one entry a frame.
    script: std::collections::VecDeque<u16>,
    /// The buttons held in the last frame, to see presses begin.
    last: u16,
}

impl Keys {
    /// For the unit named `name`: Game A / Game B / Time shortcuts on Nintendo's units.
    pub fn new(name: &str) -> Keys {
        Keys {
            ordered: name.contains("(Nintendo"),
            ..Keys::default()
        }
    }

    /// Whether the unit has the Game A / Game B / Time shortcuts.
    pub fn ordered(&self) -> bool {
        self.ordered
    }

    /// Whether `key` can be pressed now: the shortcuts need the hand put away, to count from.
    pub fn ready(&self, key: Key) -> bool {
        match key {
            Key::Next | Key::Press => true,
            Key::GameA | Key::GameB | Key::Time => self.ordered && self.hand == 0,
        }
    }

    /// A touch button tapped: queue its presses, unless it can't be pressed now (or a script is
    /// still running).
    pub fn tap(&mut self, key: Key) -> bool {
        if !self.ready(key) || !self.script.is_empty() {
            return false;
        }
        let selects = match key {
            Key::GameA => 1,
            Key::GameB => 2,
            Key::Time => 3,
            Key::Next => 1,
            Key::Press => 0,
        };
        let mut presses = vec![SELECT; selects];
        if key != Key::Next {
            presses.push(START);
        }
        for p in presses {
            self.script.extend([p; HOLD]);
            self.script.extend([0; HOLD]);
        }
        true
    }

    /// The buttons for the next frame: the player's own, `pad`, with the script's on top; and
    /// the hand followed through every press, the player's included.
    pub fn frame(&mut self, pad: u16) -> u16 {
        let held = pad | self.script.pop_front().unwrap_or(0);
        let began = held & !self.last;
        self.last = held;
        if began & START != 0 {
            self.hand = 0;
        } else if began & SELECT != 0 {
            self.hand = self.hand.saturating_add(1);
        }
        held
    }

    /// Take the hand to the unit's Zoom, `selects` presses of Select away, and press it: the
    /// core then draws the LCD alone. Queued after anything already queued.
    pub fn zoom(&mut self, selects: u32) {
        for p in std::iter::repeat_n(SELECT, selects as usize).chain([START]) {
            self.script.extend([p; HOLD]);
            self.script.extend([0; HOLD]);
        }
    }

    /// Whether no scripted presses are waiting.
    pub fn idle(&self) -> bool {
        self.script.is_empty()
    }

    /// The hand is somewhere unknown (a state was loaded).
    pub fn forget(&mut self) {
        self.hand = u32::MAX;
    }
}

/// A touch button: what it presses, its label, its box `(x, y, w, h)` on the bottom panel.
pub type Button = (Key, &'static str, (f32, f32, f32, f32));

/// The touch buttons, in the bottom panel's 1024 x 768 units: Game A, Game B and Time across the
/// middle (Nintendo units), Next key and Press under them.
pub fn buttons(ordered: bool) -> Vec<Button> {
    let mut out = Vec::new();
    if ordered {
        for (i, (key, label)) in [
            (Key::GameA, "GAME A"),
            (Key::GameB, "GAME B"),
            (Key::Time, "TIME"),
        ]
        .into_iter()
        .enumerate()
        {
            out.push((key, label, (112.0 + i as f32 * 280.0, 250.0, 240.0, 120.0)));
        }
    }
    out.push((Key::Next, "NEXT KEY", (252.0, 430.0, 240.0, 110.0)));
    out.push((Key::Press, "PRESS", (532.0, 430.0, 240.0, 110.0)));
    out
}

/// Which touch button is under `(x, y)` on the bottom panel.
pub fn hit(ordered: bool, x: f32, y: f32) -> Option<Key> {
    buttons(ordered)
        .into_iter()
        .find(|(_, _, (bx, by, bw, bh))| x >= *bx && x < bx + bw && y >= *by && y < by + bh)
        .map(|(k, _, _)| k)
}

/// The unit's name and maker, from its file's name: "Parachute (Nintendo, Wide Screen)" is
/// "Parachute" by "Nintendo, Wide Screen".
pub fn title(name: &str) -> (&str, &str) {
    match name.rfind(" (") {
        Some(at) if name.ends_with(')') => (&name[..at], &name[at + 2..name.len() - 1]),
        _ => (name, ""),
    }
}

/// The bottom panel under a unit drawn whole: its name, the touch buttons (dimmed when they
/// can't be pressed now) and how it is played.
pub fn card(name: &str, keys: &Keys) -> Vec<nibkit::menu::Draw> {
    use nibkit::menu::{colour, Draw};
    let text =
        |text: &str, size: f32, bold: bool, x: f32, y: f32, anchor: f32, colour| Draw::Text {
            text: text.into(),
            size,
            bold,
            x,
            y,
            anchor,
            colour,
        };
    let (unit, maker) = title(name);
    let mut out = vec![
        text(unit, 40.0, true, 512.0, 60.0, 0.5, colour::WHITE),
        text(maker, 26.0, false, 512.0, 114.0, 0.5, colour::DIM),
    ];
    for (key, label, (x, y, w, h)) in buttons(keys.ordered) {
        let ready = keys.ready(key);
        out.push(Draw::Round {
            x,
            y,
            w,
            h,
            radius: 22.0,
            colour: colour::PANEL,
        });
        out.push(Draw::Frame {
            x,
            y,
            w,
            h,
            radius: 22.0,
            width: 3.0,
            colour: if ready { colour::GOLD } else { colour::FAINT },
        });
        let shade = if ready { colour::WHITE } else { colour::FAINT };
        out.push(text(
            label,
            34.0,
            true,
            x + w / 2.0,
            y + h / 2.0 - 22.0,
            0.5,
            shade,
        ));
    }
    for (i, (what, does)) in CONTROLS.iter().enumerate() {
        let y = 590.0 + i as f32 * 44.0;
        out.push(text(what, 26.0, true, 330.0, y, 1.0, colour::DIM));
        out.push(text(does, 26.0, false, 352.0, y, 0.0, colour::WHITE));
    }
    out
}

/// How much to brighten an LCD shown alone, `rect` of a frame `width` pixels wide: the gain that
/// takes its pale background (the 98th percentile of the pixels' brightest channel) to
/// [`LCD_WHITE`]. The units are photographs of real LCDs, dimmer than the panel can show them;
/// a gain keeps the segments dark. Never darker, and at most [`LCD_GAIN`].
pub fn lcd_gain(rgba: &[u8], width: u32, rect: Rect) -> f32 {
    let (x, y, w, h) = rect;
    let mut counts = [0u32; 256];
    for row in y..y + h {
        let start = ((row * width + x) * 4) as usize;
        for px in rgba[start..start + (w * 4) as usize].chunks_exact(4) {
            counts[px[0].max(px[1]).max(px[2]) as usize] += 1;
        }
    }
    let total: u32 = counts.iter().sum();
    let mut seen = 0;
    let white = counts
        .iter()
        .position(|&c| {
            seen += c;
            seen * 50 >= total * 49
        })
        .unwrap_or(255)
        .max(1) as f32;
    (LCD_WHITE / white).clamp(1.0, LCD_GAIN)
}

/// Where an LCD's background is taken to when brightened, out of 255.
pub const LCD_WHITE: f32 = 225.0;
/// The most an LCD is brightened.
pub const LCD_GAIN: f32 = 2.5;

/// Brightens `rect` of a frame `width` pixels wide by `gain`, leaving alpha alone.
pub fn brighten(rgba: &mut [u8], width: u32, rect: Rect, gain: f32) {
    let lut: Vec<u8> = (0..256)
        .map(|v| (v as f32 * gain).round().min(255.0) as u8)
        .collect();
    let (x, y, w, h) = rect;
    for row in y..y + h {
        let start = ((row * width + x) * 4) as usize;
        for px in rgba[start..start + (w * 4) as usize].chunks_exact_mut(4) {
            for c in &mut px[..3] {
                *c = lut[*c as usize];
            }
        }
    }
}

/// How a unit is played on the DS+, for the menu and the bottom panel.
pub const CONTROLS: &[(&str, &str)] = &[
    ("D-pad, A, B", "The unit's own controls"),
    ("Select", "Point at the next key (Game A, Game B, Time…)"),
    ("Start", "Press the key pointed at"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_measured_units_of_the_measured_size_are_cut() {
        assert_eq!(
            fold("Donkey Kong (Nintendo, Multi Screen)", (606, 748)),
            Some(Fold::Row(375))
        );
        assert_eq!(
            fold("Donkey Kong (Nintendo, Multi Screen)", (600, 748)),
            None
        );
        assert_eq!(fold("Parachute (Nintendo, Wide Screen)", (658, 395)), None);
        assert_eq!(fold("Donkey Kong.mgw", (606, 748)), None);
    }

    #[test]
    fn a_lid_and_body_meet_at_the_panels_gap_at_one_scale() {
        let (lid, body) = split(606, 748, Fold::Row(375));
        assert_eq!(lid.src, (0, 0, 606, 375));
        assert_eq!(body.src, (0, 375, 606, 373));
        // 1024 / 606 limits the scale: both are as wide as the panel.
        assert_eq!((lid.dst.2, body.dst.2), (1024, 1024));
        assert_eq!(lid.dst.1 + lid.dst.3, 768);
        assert_eq!(body.dst.1, 0);
        // The same scale: the parts' heights keep their proportion.
        assert!((lid.dst.3 as f32 / body.dst.3 as f32 - 375.0 / 373.0).abs() < 0.01);
    }

    #[test]
    fn a_book_opens_left_page_on_top_right_page_below() {
        let (left, right) = split(973, 532, Fold::Column(484));
        assert_eq!(left.src, (0, 0, 484, 532));
        assert_eq!(right.src, (484, 0, 489, 532));
        // The height limits it here: 768 / 532.
        assert_eq!((left.dst.3, right.dst.3), (768, 768));
        assert!(left.dst.2 <= 1024 && right.dst.2 <= 1024);
        assert_eq!(left.dst.0, (1024 - left.dst.2) / 2);
    }

    fn run(keys: &mut Keys, frames: usize) -> Vec<u16> {
        (0..frames).map(|_| keys.frame(0)).collect()
    }

    #[test]
    fn game_b_is_two_selects_and_start_from_the_hand_put_away() {
        let mut keys = Keys::new("Parachute (Nintendo, Wide Screen)");
        assert!(keys.tap(Key::GameB));
        let held = run(&mut keys, 24);
        let presses: Vec<u16> = held
            .iter()
            .zip(std::iter::once(&0).chain(&held))
            .filter(|(now, before)| *now & !**before != 0)
            .map(|(now, _)| *now)
            .collect();
        assert_eq!(presses, [SELECT, SELECT, START]);
        assert!(held.iter().all(|&b| b != SELECT | START));
        assert!(keys.ready(Key::GameA));
    }

    #[test]
    fn shortcuts_wait_for_the_hand_and_other_makers_get_next_and_press() {
        let mut keys = Keys::new("Donkey Kong (Nintendo, Multi Screen)");
        // The player moves the hand with Select: Game A can't be counted from there.
        keys.frame(SELECT);
        keys.frame(0);
        assert!(!keys.ready(Key::GameA));
        assert!(!keys.tap(Key::GameA));
        assert!(keys.tap(Key::Press));
        run(&mut keys, 12);
        assert!(keys.ready(Key::GameA));
        keys.forget();
        assert!(!keys.ready(Key::Time));
        // Select after a load doesn't wrap the count round to "away".
        keys.frame(SELECT);
        keys.frame(0);
        assert!(!keys.ready(Key::GameA));

        let mut tron = Keys::new("Tron (Tomytronic)");
        assert!(!tron.ready(Key::GameA));
        assert!(tron.tap(Key::Next));
        assert_eq!(
            run(&mut tron, 6).iter().filter(|&&b| b == SELECT).count(),
            HOLD
        );
    }

    #[test]
    fn every_unit_has_a_screen_only_view() {
        // The 59 units measured: each one zooms, or has its two LCDs cut out.
        assert_eq!(ZOOM.len() + SCREENS.len(), 59);
        for (name, size, _) in ZOOM {
            assert!(screens(name, *size).is_none(), "{name}");
        }
        assert_eq!(
            zoom("Parachute (Nintendo, Wide Screen)", (658, 395)),
            Some(5)
        );
        assert_eq!(zoom("Parachute (Nintendo, Wide Screen)", (309, 193)), None);
        for (name, size, a, b) in SCREENS {
            let (w, h) = *size;
            for r in [a, b] {
                assert!(r.0 + r.2 <= w && r.1 + r.3 <= h, "{name}");
            }
            let (top, bottom) = screens(name, *size).unwrap();
            // One scale: the two LCDs keep their sizes' proportion; one of them fills a panel.
            let k = top.dst.2 as f32 / top.src.2 as f32;
            assert!((bottom.dst.2 as f32 / bottom.src.2 as f32 - k).abs() < 0.02);
            assert!([top, bottom]
                .iter()
                .any(|p| p.dst.2 == 1024 || p.dst.3 == 768));
        }
    }

    #[test]
    fn zoom_is_selects_then_start_and_puts_the_hand_away() {
        let mut keys = Keys::new("Tron (Tomytronic)");
        keys.zoom(2);
        let held = run(&mut keys, 18);
        let presses: Vec<u16> = held
            .iter()
            .zip(std::iter::once(&0).chain(&held))
            .filter(|(now, before)| *now & !**before != 0)
            .map(|(now, _)| *now)
            .collect();
        assert_eq!(presses, [SELECT, SELECT, START]);
        assert!(keys.ready(Key::Press));
    }

    #[test]
    fn a_dim_lcd_is_brightened_to_its_background_and_segments_stay_dark() {
        // A 10 x 10 frame: a 4 x 4 LCD at (2, 2), background 90 with two black segments.
        let mut rgba = vec![255u8; 10 * 10 * 4];
        let lcd = (2, 2, 4, 4);
        for row in 2..6 {
            for col in 2..6 {
                let i = (row * 10 + col) * 4;
                let v = if row == 3 && col < 4 { 20 } else { 90 };
                rgba[i..i + 3].copy_from_slice(&[v, v, v - 10]);
            }
        }
        let gain = lcd_gain(&rgba, 10, lcd);
        assert!((gain - 225.0 / 90.0).abs() < 0.01);
        brighten(&mut rgba, 10, lcd, gain);
        let at = |row: usize, col: usize| rgba[(row * 10 + col) * 4];
        assert_eq!(at(2, 2), 225);
        assert_eq!(at(3, 2), 50);
        // Outside the LCD, nothing changes; alpha neither.
        assert_eq!(at(0, 0), 255);
        assert_eq!(rgba[(2 * 10 + 2) * 4 + 3], 255);
        // A bright LCD is left as it is; a black one isn't blown up.
        assert_eq!(lcd_gain(&[240; 16], 2, (0, 0, 2, 2)), 1.0);
        assert_eq!(lcd_gain(&[0, 0, 0, 255], 1, (0, 0, 1, 1)), LCD_GAIN);
    }

    #[test]
    fn a_name_splits_into_unit_and_maker() {
        assert_eq!(
            title("Parachute (Nintendo, Wide Screen)"),
            ("Parachute", "Nintendo, Wide Screen")
        );
        assert_eq!(title("Tron (Tomytronic)"), ("Tron", "Tomytronic"));
        assert_eq!(title("Pac Man"), ("Pac Man", ""));
    }

    #[test]
    fn the_card_dims_shortcuts_the_hand_has_left() {
        let mut keys = Keys::new("Parachute (Nintendo, Wide Screen)");
        let gold = |d: &[nibkit::menu::Draw]| {
            d.iter()
                .filter(|d| {
                    matches!(d, nibkit::menu::Draw::Frame { colour, .. }
                        if *colour == nibkit::menu::colour::GOLD)
                })
                .count()
        };
        assert_eq!(gold(&card("Parachute (Nintendo, Wide Screen)", &keys)), 5);
        keys.frame(SELECT);
        assert_eq!(gold(&card("Parachute (Nintendo, Wide Screen)", &keys)), 2);
    }

    #[test]
    fn touch_buttons_are_hit_where_they_are_drawn() {
        for ordered in [true, false] {
            for (key, _, (x, y, w, h)) in buttons(ordered) {
                assert_eq!(hit(ordered, x + w / 2.0, y + h / 2.0), Some(key));
            }
        }
        assert_eq!(hit(false, 200.0, 300.0), None);
        assert_eq!(hit(true, 10.0, 10.0), None);
    }
}
