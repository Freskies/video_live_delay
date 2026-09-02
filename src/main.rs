use gstreamer as gst;

use gst::prelude::*;
use gtk::prelude::*;

use std::cell::Cell;
use std::rc::Rc;

const ROTATIONS: [&str; 4] = [
	"identity",
	"90r",
	"180",
	"90l",
];

fn set_enum_property_by_nick(
	element: &gst::Element,
	property: &str,
	nick: &str,
) {
	let property_type = element
		.property_type(property)
		.expect("Proprietà GStreamer non trovata");

	let enum_class = glib::EnumClass::new(property_type)
		.expect("La proprietà non è un enum");

	let value = enum_class
		.to_value_by_nick(nick)
		.expect("Valore enum non valido");

	element.set_property_from_value(property, &value);
}

fn rotate_video(
	video_sink: &gst::Element,
	rotation_index: &Cell<usize>,
) {
	let next = (rotation_index.get() + 1) % ROTATIONS.len();

	rotation_index.set(next);

	let rotation = ROTATIONS[next];

	set_enum_property_by_nick(
		video_sink,
		"rotate-method",
		rotation,
	);

	println!("Rotazione: {}", rotation);
}

fn main() {
	gtk::init()
		.expect("Impossibile inizializzare GTK");

	gst::init()
		.expect("Impossibile inizializzare GStreamer");

	println!("Avvio Video Live Delay...");

	// Pipeline video stabile verificata sul Raspberry Pi.
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

	/*
	 * Stato della rotazione.
	 *
	 * 0 = identity
	 * 1 = 90r
	 * 2 = 180
	 * 3 = 90l
	 */
	let rotation_index =
		Rc::new(Cell::new(0usize));

	// ---------------------------------------------------------
	// INTERFACCIA
	// ---------------------------------------------------------

	let window =
		gtk::Window::new(gtk::WindowType::Toplevel);

	window.set_title("Video Live Delay");
	window.set_default_size(1280, 800);

	/*
	 * Overlay:
	 *
	 * video sotto
	 * controlli sopra
	 */
	let overlay = gtk::Overlay::new();

	overlay.add(&video_widget);

	// Barra dei controlli.
	let controls = gtk::Box::new(
		gtk::Orientation::Horizontal,
		10,
	);

	controls.set_halign(gtk::Align::End);
	controls.set_valign(gtk::Align::End);

	controls.set_margin_end(20);
	controls.set_margin_bottom(20);

	// Pulsante rotazione.
	let rotate_button =
		gtk::Button::with_label("↻");

	// Abbastanza grande per il touchscreen.
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

				// ESC = chiudi
				if key
					== gtk::gdk::keys::constants::Escape
				{
					gtk::main_quit();
				}

				// R = ruota
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
	println!("R / pulsante ↻ = rotazione");
	println!("ESC = uscita");

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