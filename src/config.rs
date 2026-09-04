pub const VIDEO_WIDTH: usize = 1280;
pub const VIDEO_HEIGHT: usize = 720;
pub const FPS: usize = 60;

pub const INITIAL_DELAY_FRAMES: usize = FPS * 2;
pub const MAX_DELAY_FRAMES: usize = FPS * 15;
pub const DELAY_STEP_FRAMES: usize = FPS / 2; // 0.5sec