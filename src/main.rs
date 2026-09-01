use gstreamer::prelude::*;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

struct DelayEngine {
	pipeline: Option<gstreamer::Pipeline>,
	delay_sec: u64,
	rotation_deg: u32,
}

impl DelayEngine {
	fn new() -> Self {
		Self {
			pipeline: None,
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

		// Rendering hardware diretto zero-copy: 60 FPS fluidi senza passare per la CPU di GTK
		let pipeline_str = format!(
			"libcamerasrc ! \
             video/x-raw,width=1280,height=800,framerate={fps}/1 ! \
             videoflip method={flip_method} ! \
             queue max-size-buffers=0 max-size-bytes=0 max-size-time=0 \
                   min-threshold-buffers={buffer_count} min-threshold-time={delay_ns} ! \
             videoconvert ! \
             autovideosink sync=false"
		);

		if let Ok(pipe) = gstreamer::parse_launch(&pipeline_str) {
			let pipe = pipe.dynamic_cast::<gstreamer::Pipeline>().unwrap();
			let _ = pipe.set_state(gstreamer::State::Playing);
			self.pipeline = Some(pipe);
		}
	}
}

fn main() {
	gtk::init().expect("Inizializzazione GTK fallita");
	gstreamer::init().expect("Inizializzazione GStreamer fallita");

	let engine = Rc::new(RefCell::new(DelayEngine::new()));
	engine.borrow_mut().restart_pipeline();

	// Barra OSD Touch: sempre visibile in primo piano sopra il flusso video
	let window = gtk::Window::new(gtk::WindowType::Toplevel);
	window.set_title("Delay Controls");
	window.set_default_size(1280, 80);
	window.set_position(gtk::WindowPosition::Center);
	window.set_keep_above(true);
	window.set_decorated(false); // Nessuna cornice della finestra per un look OSD pulito

	let controls_box = gtk::Box::new(gtk::Orientation::Horizontal, 15);
	controls_box.set_margin_top(10);
	controls_box.set_margin_bottom(10);
	controls_box.set_margin_start(20);
	controls_box.set_margin_end(20);

	let btn_minus = gtk::Button::with_label("➖  -1s");
	btn_minus.set_size_request(150, 60);

	let lbl_status = gtk::Label::new(None);
	lbl_status.set_markup("<span font='20' weight='bold'>Delay: 3s | 0°</span>");
	lbl_status.set_hexpand(true);

	let btn_plus = gtk::Button::with_label("➕  +1s");
	btn_plus.set_size_request(150, 60);

	let btn_rotate = gtk::Button::with_label("🔄  Ruota 90°");
	btn_rotate.set_size_request(170, 60);

	let btn_close = gtk::Button::with_label("✖");
	btn_close.set_size_request(80, 60);

	controls_box.pack_start(&btn_minus, false, false, 0);
	controls_box.pack_start(&lbl_status, true, true, 0);
	controls_box.pack_start(&btn_plus, false, false, 0);
	controls_box.pack_start(&btn_rotate, false, false, 0);
	controls_box.pack_start(&btn_close, false, false, 0);

	window.add(&controls_box);

	// Eventi Touch
	{
		let eng = engine.clone();
		let lbl = lbl_status.clone();
		btn_minus.connect_clicked(move |_| {
			let mut app = eng.borrow_mut();
			if app.delay_sec > 1 {
				app.delay_sec -= 1;
				lbl.set_markup(&format!("<span font='20' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
				app.restart_pipeline();
			}
		});
	}

	{
		let eng = engine.clone();
		let lbl = lbl_status.clone();
		btn_plus.connect_clicked(move |_| {
			let mut app = eng.borrow_mut();
			app.delay_sec += 1;
			lbl.set_markup(&format!("<span font='20' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
			app.restart_pipeline();
		});
	}

	{
		let eng = engine.clone();
		let lbl = lbl_status.clone();
		btn_rotate.connect_clicked(move |_| {
			let mut app = eng.borrow_mut();
			app.rotation_deg = (app.rotation_deg + 90) % 360;
			lbl.set_markup(&format!("<span font='20' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
			app.restart_pipeline();
		});
	}

	{
		let eng = engine.clone();
		btn_close.connect_clicked(move |_| {
			if let Some(pipe) = eng.borrow_mut().pipeline.take() {
				let _ = pipe.set_state(gstreamer::State::Null);
			}
			gtk::main_quit();
		});
	}

	window.connect_delete_event(|_, _| {
		gtk::main_quit();
		glib::Propagation::Proceed
	});

	window.show_all();
	gtk::main();
}