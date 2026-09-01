use gstreamer::prelude::*;
use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	gstreamer::init()?;

	let args: Vec<String> = env::args().collect();
	let delay_seconds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
	let rotation: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(90);

	let flip_method = match rotation {
		90 => "clockwise",
		180 => "rotate-180",
		270 => "counterclockwise",
		_ => "none",
	};

	println!("==========================================");
	println!(" Video Delay System");
	println!(
		" Delay: {} s | Rotazione: {}° ({})",
		delay_seconds, rotation, flip_method
	);
	println!("==========================================");

	let delay_ns = delay_seconds * 1_000_000_000;
	let queue_max_ns = delay_ns + 2_000_000_000;

	let pipeline_str = format!(
		"libcamerasrc ! \
			video/x-raw,width=1280,height=720,framerate=30/1 ! \
			videoflip method={flip_method} ! \
			videoconvert ! \
			queue min-threshold-time={delay_ns} max-size-time={queue_max_ns} max-size-buffers=0 max-size-bytes=0 ! \
			autovideosink sync=true"
	);

	let pipeline = gstreamer::parse::launch(&pipeline_str)?;
	let pipeline = pipeline.dynamic_cast::<gstreamer::Pipeline>().unwrap();

	pipeline.set_state(gstreamer::State::Playing)?;

	let running = Arc::new(AtomicBool::new(true));
	let r = running.clone();
	ctrlc::set_handler(move || {
		println!("\nChiusura pipeline...");
		r.store(false, Ordering::SeqCst);
	})?;

	let bus = pipeline.bus().expect("Impossibile recuperare il bus");
	while running.load(Ordering::SeqCst) {
		if let Some(msg) = bus.timed_pop(gstreamer::ClockTime::from_mseconds(100)) {
			use gstreamer::MessageView;
			match msg.view() {
				MessageView::Error(err) => {
					eprintln!(
						"Errore GStreamer: {} ({})",
						err.error(),
						err.debug().unwrap_or_default()
					);
					break;
				}
				MessageView::Eos(..) => {
					println!("Fine stream.");
					break;
				}
				_ => (),
			}
		}
	}

	pipeline.set_state(gstreamer::State::Null)?;
	Ok(())
}
