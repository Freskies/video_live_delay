use std::sync::{
	atomic::{AtomicUsize, Ordering},
	Arc,
};

use crate::config::{DELAY_STEP_FRAMES, FPS, INITIAL_DELAY_FRAMES, MAX_DELAY_FRAMES};

#[derive(Clone)]
pub struct DelayController {
	frames: Arc<AtomicUsize>,
}

impl DelayController {
	pub fn new() -> Self {
		Self {
			frames: Arc::new(AtomicUsize::new(INITIAL_DELAY_FRAMES)),
		}
	}

	pub fn frames(&self) -> usize {
		self.frames.load(Ordering::Relaxed)
	}

	pub fn seconds(&self) -> f64 {
		self.frames() as f64 / FPS as f64
	}

	pub fn increase(&self) {
		let current = self.frames();
		let new_value = (current + DELAY_STEP_FRAMES).min(MAX_DELAY_FRAMES);
		self.frames.store(new_value, Ordering::Relaxed);
	}

	pub fn decrease(&self) {
		let current = self.frames();
		let new_value = current.saturating_sub(DELAY_STEP_FRAMES);
		self.frames.store(new_value, Ordering::Relaxed);
	}

	pub fn display_text(&self) -> String {
		format!("{:.1} s", self.seconds())
	}
}
