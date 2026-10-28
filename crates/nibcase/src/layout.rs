//! The DS+'s buttons laid out as a system's own pad. A core puts the console's buttons on
//! RetroPad buttons of its choosing (a Mega Drive core may put C, the button most games jump
//! with, on R) and names each in its input descriptors ("A", "C", "Mode"). For a system in
//! [`TABLES`], nibcase reads those names and puts each of the pad's buttons where it sits on
//! the system's pad: a Mega Drive's A, B and C along the DS+'s Y, B and A, as on the console's
//! own pad, whatever RetroPad buttons the core chose. The D-pad, and any button a table doesn't
//! name, go through as they are; a system without a table, or a core whose names don't match
//! it, plays as the core lays it out.
use nibcase_libretro::ffi::*;

/// A system's pad: which of the console's buttons (by the name a core gives it) each DS+ button
/// (as its RetroPad id: the printed face button, L, R, Select, Start) is, and the names a core
/// must give for the table to be used.
pub struct Table {
    pub system: &'static str,
    pub label: &'static str,
    pub keys: &'static [(u32, &'static str)],
    pub required: &'static [&'static str],
}

/// The systems laid out by name. Mega Drive: the six-button pad's A, B, C on the DS+'s left,
/// bottom and right face buttons (the row a Mega Drive pad has under the thumb), X, Y, Z on X,
/// L and R, Mode on Select.
pub const TABLES: &[Table] = &[Table {
    system: "MD",
    label: "Mega Drive",
    keys: &[
        (RETRO_DEVICE_ID_JOYPAD_Y, "A"),
        (RETRO_DEVICE_ID_JOYPAD_B, "B"),
        (RETRO_DEVICE_ID_JOYPAD_A, "C"),
        (RETRO_DEVICE_ID_JOYPAD_X, "X"),
        (RETRO_DEVICE_ID_JOYPAD_L, "Y"),
        (RETRO_DEVICE_ID_JOYPAD_R, "Z"),
        (RETRO_DEVICE_ID_JOYPAD_SELECT, "Mode"),
        (RETRO_DEVICE_ID_JOYPAD_START, "Start"),
    ],
    required: &["A", "B", "C"],
}];

/// A RetroPad id's name on the DS+ (its printed button), for the log.
fn ds_name(id: u32) -> &'static str {
    match id {
        RETRO_DEVICE_ID_JOYPAD_A => "A",
        RETRO_DEVICE_ID_JOYPAD_B => "B",
        RETRO_DEVICE_ID_JOYPAD_X => "X",
        RETRO_DEVICE_ID_JOYPAD_Y => "Y",
        RETRO_DEVICE_ID_JOYPAD_L => "L",
        RETRO_DEVICE_ID_JOYPAD_R => "R",
        RETRO_DEVICE_ID_JOYPAD_SELECT => "Select",
        RETRO_DEVICE_ID_JOYPAD_START => "Start",
        RETRO_DEVICE_ID_JOYPAD_UP => "Up",
        RETRO_DEVICE_ID_JOYPAD_DOWN => "Down",
        RETRO_DEVICE_ID_JOYPAD_LEFT => "Left",
        RETRO_DEVICE_ID_JOYPAD_RIGHT => "Right",
        RETRO_DEVICE_ID_JOYPAD_L2 => "L2",
        RETRO_DEVICE_ID_JOYPAD_R2 => "R2",
        RETRO_DEVICE_ID_JOYPAD_L3 => "L3",
        RETRO_DEVICE_ID_JOYPAD_R3 => "R3",
        _ => "?",
    }
}

/// A core's name for a button, as compared: lower case, without "button" around it.
fn plain(name: &str) -> String {
    let n = name.trim().to_ascii_lowercase();
    let n = n.strip_suffix(" button").unwrap_or(&n);
    n.strip_prefix("button ").unwrap_or(n).trim().to_owned()
}

/// How the DS+'s buttons reach the core: for each DS+ RetroPad id, the core's RetroPad ids it
/// presses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    routes: [u16; 16],
    /// What it is, for the log: the table's pad and each DS+ button's, or the core's own
    /// names.
    pub said: String,
}

impl Layout {
    /// Each DS+ button its own RetroPad button, as the core lays them out; `descriptors`
    /// (`Core::input_descriptors`) only say what each does.
    pub fn core(descriptors: &[(u32, String)]) -> Layout {
        let mut routes = [0u16; 16];
        for (id, route) in routes.iter_mut().enumerate() {
            *route = 1 << id;
        }
        let names: Vec<String> = descriptors
            .iter()
            .filter(|(id, _)| *id < 16)
            .map(|(id, what)| format!("{}={what}", ds_name(*id)))
            .collect();
        Layout {
            routes,
            said: if names.is_empty() {
                "the core's (it names none)".into()
            } else {
                format!("the core's: {}", names.join(" "))
            },
        }
    }

    /// The layout for `system` with a core that describes its buttons as `descriptors`: the
    /// system's table when it has one and the core names its required buttons, else the
    /// core's own.
    pub fn new(system: &str, descriptors: &[(u32, String)]) -> Layout {
        let mut layout = Layout::core(descriptors);
        let Some(table) = TABLES.iter().find(|t| t.system == system) else {
            return layout;
        };
        let find = |name: &str| {
            descriptors
                .iter()
                .find(|(id, what)| *id < 16 && plain(what) == plain(name))
                .map(|(id, _)| *id)
        };
        if !table.required.iter().all(|n| find(n).is_some()) {
            layout.said = format!(
                "{} (the core doesn't name {}; {})",
                table.label,
                table.required.join(", "),
                layout.said
            );
            return layout;
        }
        let mut said = Vec::new();
        for (key, name) in table.keys {
            // A button the core doesn't have presses nothing rather than something else.
            let to = find(name);
            layout.routes[*key as usize] = to.map_or(0, |id| 1 << id);
            if to.is_some() {
                said.push(format!("{}={name}", ds_name(*key)));
            }
        }
        layout.said = format!("{} pad: {}", table.label, said.join(" "));
        layout
    }

    /// The core's buttons for the DS+'s `pad` (a RetroPad mask).
    pub fn apply(&self, pad: u16) -> u16 {
        (0..16)
            .filter(|id| pad & (1 << id) != 0)
            .fold(0, |out, id| out | self.routes[id])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn described(list: &[(u32, &str)]) -> Vec<(u32, String)> {
        list.iter().map(|(id, n)| (*id, (*n).to_owned())).collect()
    }

    const A: u32 = RETRO_DEVICE_ID_JOYPAD_A;
    const B: u32 = RETRO_DEVICE_ID_JOYPAD_B;
    const X: u32 = RETRO_DEVICE_ID_JOYPAD_X;
    const Y: u32 = RETRO_DEVICE_ID_JOYPAD_Y;
    const L: u32 = RETRO_DEVICE_ID_JOYPAD_L;
    const R: u32 = RETRO_DEVICE_ID_JOYPAD_R;
    const SELECT: u32 = RETRO_DEVICE_ID_JOYPAD_SELECT;
    const START: u32 = RETRO_DEVICE_ID_JOYPAD_START;
    const UP: u32 = RETRO_DEVICE_ID_JOYPAD_UP;

    fn press(layout: &Layout, ids: &[u32]) -> u16 {
        layout.apply(ids.iter().fold(0, |m, id| m | 1 << id))
    }

    /// A Mega Drive core that puts A on Y, B on B, C on R and X, Y, Z on L, X, A (as some do):
    /// the DS+'s Y, B, A are the pad's A, B, C, its X, L, R the pad's X, Y, Z.
    #[test]
    fn a_mega_drive_pad_is_laid_out_by_its_names() {
        let core = described(&[
            (Y, "A"),
            (B, "B"),
            (R, "C"),
            (L, "X"),
            (X, "Y"),
            (A, "Z"),
            (SELECT, "Mode"),
            (START, "Start"),
        ]);
        let layout = Layout::new("MD", &core);
        assert_eq!(press(&layout, &[A]), 1 << R, "the DS+'s A is C, the jump");
        assert_eq!(press(&layout, &[Y]), 1 << Y, "Y is the pad's A");
        assert_eq!(press(&layout, &[B]), 1 << B);
        assert_eq!(press(&layout, &[X]), 1 << L, "X is X");
        assert_eq!(press(&layout, &[L]), 1 << X, "L is Y");
        assert_eq!(press(&layout, &[R]), 1 << A, "R is Z");
        assert_eq!(press(&layout, &[SELECT, START]), 1 << SELECT | 1 << START);
        assert_eq!(
            press(&layout, &[UP, A]),
            1 << UP | 1 << R,
            "the D-pad goes through"
        );
        assert_eq!(
            layout.said,
            "Mega Drive pad: Y=A B=B A=C X=X L=Y R=Z Select=Mode Start=Start"
        );
    }

    /// Names are compared without case or "button"; a three-button core has no X, Y, Z, and the
    /// DS+'s X, L, R then press nothing.
    #[test]
    fn a_three_button_core_leaves_the_rest_unpressed() {
        let core = described(&[(Y, "A Button"), (B, "button b"), (A, "c"), (START, "START")]);
        let layout = Layout::new("MD", &core);
        assert_eq!(press(&layout, &[A]), 1 << A);
        assert_eq!(press(&layout, &[Y, B]), 1 << Y | 1 << B);
        assert_eq!(press(&layout, &[X, L, R, SELECT]), 0);
        assert_eq!(press(&layout, &[START]), 1 << START);
    }

    /// A core that doesn't name A, B and C plays as it lays itself out; so does a system
    /// without a table.
    #[test]
    fn otherwise_the_core_lays_out_its_own_pad() {
        let odd = described(&[(B, "Jump"), (A, "Fire")]);
        let layout = Layout::new("MD", &odd);
        assert_eq!(press(&layout, &[A, R]), 1 << A | 1 << R);
        assert!(layout
            .said
            .starts_with("Mega Drive (the core doesn't name A, B, C;"));
        let nes = Layout::new("FC", &described(&[(B, "B"), (A, "A")]));
        assert_eq!(press(&nes, &[A, B, X]), 1 << A | 1 << B | 1 << X);
        assert_eq!(nes.said, "the core's: B=B A=A");
        assert_eq!(Layout::new("SFC", &[]).said, "the core's (it names none)");
    }
}
