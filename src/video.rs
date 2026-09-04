use gstreamer as gst;
use gstreamer_app as gst_app;

use gst::prelude::*;
use gtk::prelude::*;

use crate::config::{FPS, MAX_DELAY_FRAMES};

use crate::delay::DelayController;
use crate::pipelines;
use crate::video_buffer::VideoBuffer;

pub struct VideoSystem {
	capture_pipeline: gst::Pipeline,
	playback_pipeline: gst::Pipeline,

	video_sink: gst::Element,
	video_widget: gtk::Widget,
}

impl VideoSystem {
	// PUBLIC API

	pub fn new(delay: DelayController) -> Self {
		let (capture_pipeline, capture_sink) = Self::create_capture_pipeline();

		let (playback_pipeline, playback_src, video_sink, video_widget) =
			Self::create_playback_pipeline();

		Self::setup_delay(&capture_sink, playback_src, delay);

		Self {
			capture_pipeline,
			playback_pipeline,
			video_sink,
			video_widget,
		}
	}

	pub fn video_widget(&self) -> gtk::Widget {
		self.video_widget.clone()
	}

	pub fn video_sink(&self) -> gst::Element {
		self.video_sink.clone()
	}

	pub fn start(&self) {
		println!("Starting playback pipeline...");
		Self::start_pipeline(&self.playback_pipeline);
		println!("Starting capture pipeline...");
		Self::start_pipeline(&self.capture_pipeline);
	}

	pub fn stop(&self) {
		println!("Stopping capture pipeline...");
		Self::stop_pipeline(&self.capture_pipeline);
		println!("Stopping playback pipeline...");
		Self::stop_pipeline(&self.playback_pipeline);
	}

	// PRIVATE

	fn create_capture_pipeline() -> (gst::Pipeline, gst_app::AppSink) {
		let pipeline = gst::parse_launch(&pipelines::capture())
			.expect("Cannot create capture pipeline")
			.downcast::<gst::Pipeline>()
			.expect("capture_pipeline is not a 'Pipeline' wtf?");

		let sink = pipeline
			.by_name("capture_sink")
			.expect("capture_sink not found")
			.downcast::<gst_app::AppSink>()
			.expect("capture_sink is not an 'AppSink' wtf?");

		(pipeline, sink)
	}

	fn create_playback_pipeline() -> (gst::Pipeline, gst_app::AppSrc, gst::Element, gtk::Widget) {
		let pipeline = gst::parse_launch(pipelines::PLAYBACK)
			.expect("Cannot create playback pipeline")
			.downcast::<gst::Pipeline>()
			.expect("playback_pipeline is not a 'Pipeline' wtf?");

		let source = pipeline
			.by_name("playback_src")
			.expect("playback_src not found")
			.downcast::<gst_app::AppSrc>()
			.expect("playback_src is not an 'AppSink' wtf?");

		let sink = pipeline
			.by_name("video_sink")
			.expect("video_sink not found");

		let widget = sink.property::<gtk::Widget>("widget");

		(pipeline, source, sink, widget)
	}

	fn start_pipeline(pipeline: &gst::Pipeline) {
		pipeline
			.set_state(gst::State::Playing)
			.expect("Cannot start the pipeline");
	}

	fn stop_pipeline(pipeline: &gst::Pipeline) {
		pipeline
			.set_state(gst::State::Null)
			.expect("Cannot stop the pipeline");
	}

	fn setup_delay(
		capture_sink: &gst_app::AppSink,
		playback_src: gst_app::AppSrc,
		delay: DelayController,
	) {
		let mut video_buffer = VideoBuffer::new(MAX_DELAY_FRAMES);

		let mut first_frame = true;
		let mut playback_started = false;

		capture_sink.set_callbacks(
			gst_app::AppSinkCallbacks::builder()
				.new_sample(move |appsink| {
					let input_sample = Self::pull_sample(appsink)?;
					if first_frame {
						Self::print_first_frame(&input_sample);
						first_frame = false;
					}

					video_buffer.push(&input_sample)?;

					let target_delay = delay.frames();
					let Some(output_sample) = video_buffer.delayed_sample(target_delay) else {
						return Ok(gst::FlowSuccess::Ok);
					};

					if !playback_started {
						Self::print_playback_started(target_delay);
						playback_started = true;
					}

					Self::push_to_playback(&playback_src, &output_sample)?;
					Ok(gst::FlowSuccess::Ok)
				})
				.build(),
		);
	}

	fn pull_sample(appsink: &gst_app::AppSink) -> Result<gst::Sample, gst::FlowError> {
		appsink.pull_sample().map_err(|_| gst::FlowError::Eos)
	}

	fn push_to_playback(
		playback_src: &gst_app::AppSrc,
		sample: &gst::Sample,
	) -> Result<(), gst::FlowError> {
		playback_src.push_sample(sample).map_err(|err| {
			eprintln!("Error in push_sample: {:?}", err);
			err
		})?;

		Ok(())
	}

	// DEBUG

	fn print_first_frame(sample: &gst::Sample) {
		println!("First frame received by the camera");
		if let Some(caps) = sample.caps() {
			println!("Camera format: {}", caps);
		}
	}

	fn print_playback_started(delay_frames: usize) {
		let seconds = delay_frames as f64 / FPS as f64;
		println!("Playback started with {:.1}s delay", seconds);
	}
}
