use glib::prelude::*;
use gstreamer::prelude::*;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

struct DelayApp {
	pipeline: Option<gstreamer::Pipeline>,
	video_sink: gstreamer::Element,
	delay_sec: u64,
	rotation_deg: u32,
}

impl DelayApp {
	fn new(video_sink: gstreamer::Element) -> Self {
		Self {
			pipeline: None,
			video_sink,
			delay_sec: 3,
			rotation_deg: 0,
		}
	}

	fn restart_pipeline(&mut self) {
		if let Some(pipe) = self.pipeline.take() {
			let _ = pipe.set_state(gstreamer::State::Null);
		}

		let fps = 60;
		let buffer_count = self.delay_sec * fps;
		let delay_ns = self.delay_sec * 1_000_000_000;

		let flip_method = match self.rotation_deg % 360 {
			90 => "clockwise",
			180 => "rotate-180",
			270 => "counterclockwise",
			_ => "none",
		};

		let pipe = gstreamer::Pipeline::new();
		let src = gstreamer::ElementFactory::make("libcamerasrc").build().unwrap();
		let filter1 = gstreamer::ElementFactory::make("capsfilter").build().unwrap();
		let flip = gstreamer::ElementFactory::make("videoflip").build().unwrap();
		let queue = gstreamer::ElementFactory::make("queue").build().unwrap();
		let conv = gstreamer::ElementFactory::make("videoconvert").build().unwrap();

		let caps = gstreamer::Caps::builder("video/x-raw")
			.field("width", 1280)
			.field("height", 800)
			.field("framerate", gstreamer::Fraction::new(60, 1))
			.build();
		glib::ObjectExt::set_property(&filter1, "caps", &caps);

		flip.set_property_from_str("method", flip_method);

		glib::ObjectExt::set_property(&queue, "max-size-buffers", 0u32);
		glib::ObjectExt::set_property(&queue, "max-size-bytes", 0u32);
		glib::ObjectExt::set_property(&queue, "max-size-time", 0u64);
		glib::ObjectExt::set_property(&queue, "min-threshold-buffers", buffer_count as u32);
		glib::ObjectExt::set_property(&queue, "min-threshold-time", delay_ns);

		pipe.add_many([&src, &filter1, &flip, &queue, &conv, &self.video_sink]).unwrap();
		gstreamer::Element::link_many([&src, &filter1, &flip, &queue, &conv, &self.video_sink]).unwrap();

		pipe.set_state(gstreamer::State::Playing).unwrap();
		self.pipeline = Some(pipe);
	}
}

fn main() {
	gtk::init().expect("Inizializzazione GTK fallita");
	gstreamer::init().expect("Inizializzazione GStreamer fallita");

	let window = gtk::Window::new(gtk::WindowType::Toplevel);
	window.set_title("Video Live Delay");
	window.fullscreen();

	let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);

	let sink = gstreamer::ElementFactory::make("gtksink")
		.build()
		.expect("Plugin gtksink non trovato. Installa gstreamer1.0-gtk3");

	let video_widget: gtk::Widget = glib::ObjectExt::property(&sink, "widget");
	video_widget.set_hexpand(true);
	video_widget.set_vexpand(true);

	vbox.pack_start(&video_widget, true, true, 0);

	let controls_box = gtk::Box::new(gtk::Orientation::Horizontal, 15);
	controls_box.set_margin_top(10);
	controls_box.set_margin_bottom(10);
	controls_box.set_margin_start(20);
	controls_box.set_margin_end(20);

	let btn_minus = gtk::Button::with_label("➖  -1s Delay");
	btn_minus.set_size_request(160, 60);

	let lbl_status = gtk::Label::new(None);
	lbl_status.set_markup("<span font='18' weight='bold'>Delay: 3s | 0°</span>");
	lbl_status.set_hexpand(true);

	let btn_plus = gtk::Button::with_label("➕  +1s Delay");
	btn_plus.set_size_request(160, 60);

	let btn_rotate = gtk::Button::with_label("🔄  Ruota 90°");
	btn_rotate.set_size_request(160, 60);

	let btn_close = gtk::Button::with_label("❌ Chiudi");
	btn_close.set_size_request(120, 60);

	controls_box.pack_start(&btn_minus, false, false, 0);
	controls_box.pack_start(&lbl_status, true, true, 0);
	controls_box.pack_start(&btn_plus, false, false, 0);
	controls_box.pack_start(&btn_rotate, false, false, 0);
	controls_box.pack_start(&btn_close, false, false, 0);

	vbox.pack_end(&controls_box, false, false, 0);
	window.add(&vbox);

	let app_state = Rc::new(RefCell::new(DelayApp::new(sink)));
	app_state.borrow_mut().restart_pipeline();

	{
		let state = app_state.clone();
		let lbl = lbl_status.clone();
		btn_minus.connect_clicked(move |_| {
			let mut app = state.borrow_mut();
			if app.delay_sec > 1 {
				app.delay_sec -= 1;
				lbl.set_markup(&format!("<span font='18' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
				app.restart_pipeline();
			}
		});
	}

	{
		let state = app_state.clone();
		let lbl = lbl_status.clone();
		btn_plus.connect_clicked(move |_| {
			let mut app = state.borrow_mut();
			app.delay_sec += 1;
			lbl.set_markup(&format!("<span font='18' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
			app.restart_pipeline();
		});
	}

	{
		let state = app_state.clone();
		let lbl = lbl_status.clone();
		btn_rotate.connect_clicked(move |_| {
			let mut app = state.borrow_mut();
			app.rotation_deg = (app.rotation_deg + 90) % 360;
			lbl.set_markup(&format!("<span font='18' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
			app.restart_pipeline();
		});
	}

	btn_close.connect_clicked(|_| {
		gtk::main_quit();
	});

	window.connect_delete_event(|_, _| {
		gtk::main_quit();
		glib::Propagation::Proceed
	});

	window.show_all();
	gtk::main();
}