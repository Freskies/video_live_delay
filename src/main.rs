pub mod config;
pub mod delay;
pub mod pipelines;
pub mod ui;
pub mod video;
pub mod video_buffer;

use gstreamer as gst;
use gtk::prelude::*;

use delay::DelayController;
use video::VideoSystem;

fn main() {
	gtk::init().expect("Failed to initialize GTK");
	gst::init().expect("Failed to initialize GStreamer");

	println!("==============================");
	println!("     VIDEO LIVE DELAY");
	println!("==============================");

	let delay = DelayController::new();
	println!("Start delay: {}", delay.display_text());

	let video = VideoSystem::new(delay.clone());
	let window = ui::build_window(video.video_widget(), video.video_sink(), delay);

	window.show_all();
	window.fullscreen();

	video.start();
	gtk::main();
	video.stop();
	println!("Stopped");
}
