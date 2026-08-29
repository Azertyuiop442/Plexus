
pub mod ansi;
pub mod aiprefs_tests;
pub mod loader;
pub mod model;
pub mod persistence;
pub mod render;

#[allow(unused_imports)]
pub use loader::load_modal;
#[allow(unused_imports)]
pub use model::{Modal, ModalRow, ModalStep, is_reasoning_model, update_paired_effort_rows};
#[allow(unused_imports)]
pub use render::{
    dim_background, modal_choice_rows, modal_rect, modal_stack_areas, render_modal,
    row_content_width,
};

pub mod picker;
pub mod auto_retry;
#[allow(unused_imports)]
pub use picker::{ModelPicker, PICKER_CATEGORIES};
#[allow(unused_imports)]
pub use auto_retry::open_auto_retry_modal;

