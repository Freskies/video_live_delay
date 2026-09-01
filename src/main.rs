use gstreamer::prelude::*;
use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

fn build_pipeline(delay_sec: u64, rotation_deg: u32) -> Result<gstreamer::Pipeline, Box<dyn std::error::Error>> {
	let fps = 60;
	let buffer_count = delay_sec * fps;
	let delay_ns = delay_sec * 1_000_000_000;

	let flip_method = match rotation_deg % 360 {
		90 => "clockwise",
		180 => "rotate-180",
		270 => "counterclockwise",
		_ => "none",
	};

	let pipeline_str = format!(
		"libcamerasrc ! \
         video/x-raw,width=1280,height=720,framerate={fps}/1 ! \
         videoflip method={flip_method} ! \
         queue max-size-buffers=0 max-size-bytes=0 max-size-time=0 \
               min-threshold-buffers={buffer_count} min-threshold-time={delay_ns} ! \
         videoconvert ! \
         autovideosink sync=false"
	);

	let pipeline = gstreamer::parse::launch(&pipeline_str)?;
	Ok(pipeline.dynamic_cast::<gstreamer::Pipeline>().unwrap())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	gstreamer::init()?;

	let delay = Arc::new(AtomicU64::new(3));
	let rotation = Arc::new(AtomicU32::new(90));
	let running = Arc::new(AtomicBool::new(true));

	println!("==========================================");
	println!(" Video Live Delay Controller");
	println!(" [r] Ruota 90° | [+] Aumenta Delay | [-] Riduci Delay | [q] Esci");
	println!("==========================================");

	let mut current_pipeline = build_pipeline(delay.load(Ordering::SeqCst), rotation.load(Ordering::SeqCst))?;
	current_pipeline.set_state(gstreamer::State::Playing)?;

	// Thread per input interattivo da tastiera/console
	let d_clone = delay.clone();
	let r_clone = rotation.clone();
	let run_clone = running.clone();
	let reload_trigger = Arc::new(AtomicBool::new(false));
	let reload_t = reload_trigger.clone();

	thread::spawn(move || {
		let stdin = io::stdin();
		for line in stdin.lock().lines() {
			if let Ok(cmd) = line {
				let trimmed = cmd.trim();
				match trimmed {
					"r" | "R" => {
						let cur = r_clone.load(Ordering::SeqCst);
						r_clone.store((cur + 90) % 360, Ordering::SeqCst);
						println!("-> Nuova rotazione: {}°", r_clone.load(Ordering::SeqCst));
						reload_t.store(true, Ordering::SeqCst);
					}
					"+" => {
						d_clone.fetch_add(1, Ordering::SeqCst);
						println!("-> Delay impostato a: {} s", d_clone.load(Ordering::SeqCst));
						reload_t.store(true, Ordering::SeqCst);
					}
					"-" => {
						let cur = d_clone.load(Ordering::SeqCst);
						if cur > 1 {
							d_clone.store(cur - 1, Ordering::SeqCst);
							println!("-> Delay impostato a: {} s", d_clone.load(Ordering::SeqCst));
							reload_t.store(true, Ordering::SeqCst);
						}
					}
					"q" | "Q" => {
						run_clone.store(false, Ordering::SeqCst);
						break;
					}
					_ => (),
				}
			}
		}
	});

	let r_sig = running.clone();
	ctrlc::set_handler(move || {
		r_sig.store(false, Ordering::SeqCst);
	})?;

	while running.load(Ordering::SeqCst) {
		if reload_trigger.swap(false, Ordering::SeqCst) {
			let _ = current_pipeline.set_state(gstreamer::State::Null);
			let cur_d = delay.load(Ordering::SeqCst);
			let cur_r = rotation.load(Ordering::SeqCst);
			if let Ok(new_pipe) = build_pipeline(cur_d, cur_r) {
				current_pipeline = new_pipe;
				let _ = current_pipeline.set_state(gstreamer::State::Playing);
			}
		}
		thread::sleep(std::time::Duration::from_millis(100));
	}

	let _ = current_pipeline.set_state(gstreamer::State::Null);
	Ok(())
}