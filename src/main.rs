use gstreamer::prelude::*;
use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	gstreamer::init()?;

	// Parametri: ritardo in secondi (default 3)
	let args: Vec<String> = env::args().collect();
	let delay_seconds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);

	println!("==========================================");
	println!(" Video Delay System - High Performance Mode");
	println!(
		" Delay: {} s | Fullscreen KMS Hardware Rendering",
		delay_seconds
	);
	println!("==========================================");

	let delay_ns = delay_seconds * 1_000_000_000;
	let queue_max_ns = delay_ns + 2_000_000_000;

	// Pipeline ottimizzata per 60fps con zero carico CPU:
	// 1. Acquisizione diretta 1280x720 @ 60fps in formato NV12 (nativo GPU Pi 4)
	// 2. Buffer in memoria ad anello
	// 3. kmssink per fullscreen hardware diretto senza passare dal desktop
	let pipeline_str = format!(
		"libcamerasrc ! \
			video/x-raw,format=NV12,width=1280,height=720,framerate=60/1 ! \
			queue min-threshold-time={delay_ns} max-size-time={queue_max_ns} max-size-buffers=0 max-size-bytes=0 ! \
			glimagesink sync=false"
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
				MessageView::Eos(..) => break,
				_ => (),
			}
		}
	}

	pipeline.set_state(gstreamer::State::Null)?;
	Ok(())
}
