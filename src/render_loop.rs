
pub const MIN_FRAME_MS: u64 = 4;

pub const FRAME_MS: u64 = 33;

pub const IDLE_CADENCE_MS: u64 = 200;

pub const LOADING_DRAW_MS: u64 = 150;
pub fn viewport_size<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
) -> Result<ratatui::layout::Size, B::Error> {
    terminal.size()
}

#[allow(dead_code)]
pub fn should_draw(dirty: bool, elapsed_since_last_draw: std::time::Duration) -> bool {
    if dirty {
        elapsed_since_last_draw >= std::time::Duration::from_millis(MIN_FRAME_MS)
    } else {
        elapsed_since_last_draw >= std::time::Duration::from_millis(IDLE_CADENCE_MS)
    }
}

pub fn should_draw_split(
    input_dirty: bool,
    output_dirty: bool,
    elapsed_since_last_draw: std::time::Duration,
) -> bool {
    let elapsed_ms = elapsed_since_last_draw.as_millis() as u64;
    if input_dirty {
        elapsed_ms >= MIN_FRAME_MS
    } else if output_dirty {
        elapsed_ms >= FRAME_MS
    } else {
        elapsed_ms >= IDLE_CADENCE_MS
    }
}

