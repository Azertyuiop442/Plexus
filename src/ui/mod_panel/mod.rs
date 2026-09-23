
pub mod input;
pub mod render;
pub mod state;

pub use input::{handle_key, PanelAction};
pub use render::render_panel;
pub use state::{PanelState, PanelView};

