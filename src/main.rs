use glib::prelude::*;
use glib::translate::ToGlibPtr;
use gstreamer::prelude::*;
use gstreamer_video::prelude::*;
use gtk::prelude::*;
use std::cell::RefCell;
use std::os::raw::{c_ulong, c_void};
use std::rc::Rc;

extern "C" {
	fn gdk_x11_window_get_xid(window: *mut c_void) -> c_ulong;
}

struct DelayApp {
	pipeline: Option<gstreamer::Pipeline>,
	delay_sec: u64,
	rotation_deg: u32,
	window_xid: usize,
}

impl DelayApp {
	fn new(window_xid: usize) -> Self {
		Self {
			pipeline: None,
			delay_sec: 3,
			rotation_deg: 0,
			window_xid,
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

		// Pipeline hardware zero-copy a 60 FPS
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

			let target_xid = self.window_xid;
			if target_xid != 0 {
				if let Some(bus) = pipe.bus() {
					bus.set_sync_handler(move |_bus, msg| {
						if gstreamer_video::is_video_overlay_prepare_window_handle_message(msg) {
							if let Some(overlay) = msg.src().and_then(|s| s.dynamic_cast::<gstreamer_video::VideoOverlay>().ok()) {
								unsafe {
									overlay.set_window_handle(target_xid);
								}
							}
						}
						gstreamer::BusSyncReply::Pass
					});
				}
			}

			let _ = pipe.set_state(gstreamer::State::Playing);
			self.pipeline = Some(pipe);
		}
	}
}

fn main() {
	gtk::init().expect("Inizializzazione GTK fallita");
	gstreamer::init().expect("Inizializzazione GStreamer fallita");

	let window = gtk::Window::new(gtk::WindowType::Toplevel);
	window.set_title("Video Live Delay");
	window.fullscreen();

	let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

	let drawing_area = gtk::DrawingArea::new();
	drawing_area.set_hexpand(true);
	drawing_area.set_vexpand(true);
	main_box.pack_start(&drawing_area, true, true, 0);

	let controls_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
	controls_box.set_margin_top(6);
	controls_box.set_margin_bottom(6);
	controls_box.set_margin_start(16);
	controls_box.set_margin_end(16);

	let btn_minus = gtk::Button::with_label("➖  -1s");
	btn_minus.set_size_request(140, 56);

	let lbl_status = gtk::Label::new(None);
	lbl_status.set_markup("<span font='18' weight='bold'>Delay: 3s | 0°</span>");
	lbl_status.set_hexpand(true);

	let btn_plus = gtk::Button::with_label("➕  +1s");
	btn_plus.set_size_request(140, 56);

	let btn_rotate = gtk::Button::with_label("🔄  Ruota 90°");
	btn_rotate.set_size_request(150, 56);

	let btn_close = gtk::Button::with_label("✖");
	btn_close.set_size_request(70, 56);

	controls_box.pack_start(&btn_minus, false, false, 0);
	controls_box.pack_start(&lbl_status, true, true, 0);
	controls_box.pack_start(&btn_plus, false, false, 0);
	controls_box.pack_start(&btn_rotate, false, false, 0);
	controls_box.pack_start(&btn_close, false, false, 0);

	main_box.pack_end(&controls_box, false, false, 0);
	window.add(&main_box);

	let app_state = Rc::new(RefCell::new(None::<DelayApp>));

	// Estrae l'XID quando la finestra viene mappata a schermo
	{
		let app_ref = app_state.clone();
		let area = drawing_area.clone();
		window.connect_map(move |_| {
			let xid = if let Some(gdk_win) = area.window() {
				let ptr: *mut c_void = gdk_win.as_ref().to_glib_none().0 as *mut c_void;
				unsafe { gdk_x11_window_get_xid(ptr) as usize }
			} else {
				0
			};

			let mut app = DelayApp::new(xid);
			app.restart_pipeline();
			*app_ref.borrow_mut() = Some(app);
		});
	}

	// Gestione touch Meno
	{
		let state = app_state.clone();
		let lbl = lbl_status.clone();
		btn_minus.connect_clicked(move |_| {
			if let Some(ref mut app) = *state.borrow_mut() {
				if app.delay_sec > 1 {
					app.delay_sec -= 1;
					lbl.set_markup(&format!("<span font='18' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
					app.restart_pipeline();
				}
			}
		});
	}

	// Gestione touch Più
	{
		let state = app_state.clone();
		let lbl = lbl_status.clone();
		btn_plus.connect_clicked(move |_| {
			if let Some(ref mut app) = *state.borrow_mut() {
				app.delay_sec += 1;
				lbl.set_markup(&format!("<span font='18' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
				app.restart_pipeline();
			}
		});
	}

	// Gestione touch Rotazione
	{
		let state = app_state.clone();
		let lbl = lbl_status.clone();
		btn_rotate.connect_clicked(move |_| {
			if let Some(ref mut app) = *state.borrow_mut() {
				app.rotation_deg = (app.rotation_deg + 90) % 360;
				lbl.set_markup(&format!("<span font='18' weight='bold'>Delay: {}s | {}°</span>", app.delay_sec, app.rotation_deg));
				app.restart_pipeline();
			}
		});
	}

	// Gestione touch Chiusura
	{
		let state = app_state.clone();
		btn_close.connect_clicked(move |_| {
			if let Some(ref mut app) = *state.borrow_mut() {
				if let Some(pipe) = app.pipeline.take() {
					let _ = pipe.set_state(gstreamer::State::Null);
				}
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