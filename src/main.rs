use gstreamer as gst;
use gstreamer_video as gst_video;

use gst::prelude::*;
use gtk::prelude::*;

use std::cell::Cell;
use std::rc::Rc;

const ROTATIONS: [gst_video::VideoOrientationMethod; 4] = [
	gst_video::VideoOrientationMethod::Identity,
	gst_video::VideoOrientationMethod::_90r,
	gst_video::VideoOrientationMethod::_180,
	gst_video::VideoOrientationMethod::_90l,
];

fn rotate_video(
	video_sink: &gst::Element,
	rotation_index: &Cell<usize>,
) {
	let next = (rotation_index.get() + 1) % ROTATIONS.len();

	rotation_index.set(next);

	let rotation = ROTATIONS[next];

	video_sink.set_property("rotate-method", rotation);

	println!("Rotazione: {:?}", rotation);
}

fn main() {
	gtk::init()
		.expect("Impossibile inizializzare GTK");

	gst::init()
		.expect("Impossibile inizializzare GStreamer");

	println!("Avvio Video Live Delay...");

	// Pipeline verificata sul Raspberry Pi:
	// 1280x720 @ 60 FPS, fluida e con rendering OpenGL.
	let pipeline_description = r#"
        libcamerasrc !
        video/x-raw,width=1280,height=720,framerate=60/1 !
        queue !
        glupload !
        glcolorconvert !
        gtkglsink name=video_sink
    "#;

	let pipeline = gst::parse_launch(pipeline_description)
		.expect("Impossibile creare la pipeline")
		.downcast::<gst::Pipeline>()
		.expect("L'elemento creato non è una Pipeline");

	let video_sink = pipeline
		.by_name("video_sink")
		.expect("video_sink non trovato");

	let video_widget =
		video_sink.property::<gtk::Widget>("widget");

	// 0 = 0°
	// 1 = 90°
	// 2 = 180°
	// 3 = 270°
	let rotation_index =
		Rc::new(Cell::new(0usize));

	// ---------------------------------------------------------
	// FINESTRA
	// ---------------------------------------------------------

	let window =
		gtk::Window::new(gtk::WindowType::Toplevel);

	window.set_title("Video Live Delay");
	window.set_default_size(1280, 800);

	// Overlay:
	// video sotto, controlli sopra.
	let overlay = gtk::Overlay::new();

	overlay.add(&video_widget);

	// ---------------------------------------------------------
	// BARRA CONTROLLI
	// ---------------------------------------------------------

	let controls = gtk::Box::new(
		gtk::Orientation::Horizontal,
		10,
	);

	controls.set_halign(gtk::Align::End);
	controls.set_valign(gtk::Align::End);

	controls.set_margin_end(20);
	controls.set_margin_bottom(20);

	let rotate_button =
		gtk::Button::with_label("↻");

	rotate_button.set_size_request(90, 70);

	controls.pack_start(
		&rotate_button,
		false,
		false,
		0,
	);

	overlay.add_overlay(&controls);

	window.add(&overlay);

	// ---------------------------------------------------------
	// PULSANTE ROTAZIONE
	// ---------------------------------------------------------

	{
		let video_sink = video_sink.clone();
		let rotation_index =
			rotation_index.clone();

		rotate_button.connect_clicked(move |_| {
			rotate_video(
				&video_sink,
				&rotation_index,
			);
		});
	}

	// ---------------------------------------------------------
	// TASTIERA
	// ---------------------------------------------------------

	{
		let video_sink = video_sink.clone();
		let rotation_index =
			rotation_index.clone();

		window.connect_key_press_event(
			move |_, event| {

				let key = event.keyval();

				if key
					== gtk::gdk::keys::constants::Escape
				{
					gtk::main_quit();
				}

				if key
					== gtk::gdk::keys::constants::r
					|| key
					== gtk::gdk::keys::constants::R
				{
					rotate_video(
						&video_sink,
						&rotation_index,
					);
				}

				glib::Propagation::Proceed
			},
		);
	}

	window.connect_destroy(|_| {
		gtk::main_quit();
	});

	// ---------------------------------------------------------
	// AVVIO
	// ---------------------------------------------------------

	window.show_all();
	window.fullscreen();

	pipeline
		.set_state(gst::State::Playing)
		.expect("Impossibile avviare la pipeline");

	println!("Pipeline avviata");
	println!("1280x720 @ 60 FPS");
	println!("R / ↻ = ruota");
	println!("ESC = esci");

	gtk::main();

	// ---------------------------------------------------------
	// ARRESTO
	// ---------------------------------------------------------

	println!("Arresto pipeline...");

	pipeline
		.set_state(gst::State::Null)
		.expect("Impossibile arrestare la pipeline");

	println!("Terminato");
}