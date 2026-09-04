use gstreamer as gst;
use std::collections::VecDeque;

pub struct VideoBuffer {
	history: VecDeque<gst::Sample>,
	max_delay_frames: usize,
}

impl VideoBuffer {
	pub fn new(max_delay_frames: usize) -> Self {
		Self {
			history: VecDeque::with_capacity(max_delay_frames + 1),
			max_delay_frames,
		}
	}

	pub fn push(&mut self, input_sample: &gst::Sample) -> Result<(), gst::FlowError> {
		let stored_sample = Self::copy_sample(input_sample)?;
		self.history.push_back(stored_sample);
		self.trim();
		Ok(())
	}

	pub fn delayed_sample(&self, delay_frames: usize) -> Option<gst::Sample> {
		// if there aren't enough frames in the buffer
		if self.history.len() <= delay_frames {
			return None;
		}

		let index = self.history.len() - 1 - delay_frames;
		self.history.get(index).cloned()
	}

	pub fn len(&self) -> usize {
		self.history.len()
	}

	fn trim(&mut self) {
		while self.history.len() > self.max_delay_frames + 1 {
			self.history.pop_front();
		}
	}

	fn copy_sample(input_sample: &gst::Sample) -> Result<gst::Sample, gst::FlowError> {
		let input_buffer = input_sample.buffer().ok_or(gst::FlowError::Error)?;

		let caps = input_sample
			.caps()
			.ok_or(gst::FlowError::NotNegotiated)?
			.to_owned();

		let mut copied_buffer = input_buffer.copy_deep().map_err(|err| {
			eprintln!("Error copy_deep: {}", err);
			gst::FlowError::Error
		})?;

		{
			let buffer = copied_buffer
				.get_mut()
				.expect("Cannot modify the copied buffer");
			buffer.set_pts(None::<gst::ClockTime>);
			buffer.set_dts(None::<gst::ClockTime>);
		}

		Ok(gst::Sample::builder()
			.buffer(&copied_buffer)
			.caps(&caps)
			.build())
	}
}
