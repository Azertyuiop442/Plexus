
pub mod contract;
pub mod effort;
pub mod load;

pub const STALE_BRIDGE_MS: u64 = 60_000;

pub fn bridge_is_stale(updated_at: Option<u64>, started_at_ms: u64, now_ms: u64) -> bool {
    match updated_at {
        Some(t) => t < started_at_ms || now_ms.saturating_sub(t) > STALE_BRIDGE_MS,
        None => true,
    }
}

#[allow(unused_imports)]
pub use contract::*;
#[allow(unused_imports)]
pub use load::*;

use ratatui::style::Color;

use crate::theme::Palette;

pub fn color_from_name(p: &Palette, name: &str) -> Color {
    match name {
        "green" => p.green,
        "yellow" => p.yellow,
        "red" => p.red,
        "blue" => p.blue,
        "accent" => p.accent,
        "text" => p.text,
        "muted" => p.overlay0,
        "cyan" => p.blue,
        "purple" | "mauve" => p.mauve,
        "peach" | "orange" => p.peach,
        _ => p.text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_names_map_to_palette() {
        let p = Palette::dark();
        assert_eq!(color_from_name(&p, "yellow"), p.yellow);
        assert_eq!(color_from_name(&p, "red"), p.red);
        assert_eq!(color_from_name(&p, "unknown"), p.text);
    }
}

