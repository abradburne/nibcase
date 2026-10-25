//! The menu's Connections page: Wi-Fi and Bluetooth on or off, and the paired Bluetooth devices
//! (headphones) connected or disconnected with one press, without leaving the game. It is
//! nibkit's ready-made page (`nibkit::menu::connections`, feature `net-control`, nibOS only), the
//! same as DSlot's and every app's. Built without it (the stock OS, a computer) there is no such
//! page: `Connections::start` gives `None`, and the menu shows no tab for it.

#[cfg(feature = "net-control")]
pub use nibkit::menu::connections::{Connections, HINT};

#[cfg(not(feature = "net-control"))]
mod none {
    use nibkit::menu::Row;

    /// The page's hint line (never shown without the page).
    pub const HINT: &str = "";

    /// Never made: this build has no Wi-Fi or Bluetooth control.
    pub enum Connections {}

    impl Connections {
        pub fn start() -> Option<Connections> {
            None
        }
        pub fn refresh(&mut self) {
            match *self {}
        }
        pub fn title(&self) -> String {
            match *self {}
        }
        pub fn rows(&self) -> Vec<Row> {
            match *self {}
        }
        pub fn act(&mut self, _row: usize) -> bool {
            match *self {}
        }
    }
}
#[cfg(not(feature = "net-control"))]
pub use none::{Connections, HINT};
