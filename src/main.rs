use gstreamer as gst;

use gst::prelude::*;
use gtk::prelude::*;

fn main() {
	// Inizializzazione
	gtk::init().expect("Impossibile inizializzare GTK");
	gst::init().expect("Impossibile inizializzare GStreamer");

	println!("Avvio Video Live Delay...");

	// Questa è ESATTAMENTE la pipeline che abbiamo già testato
	// sul Raspberry e che sappiamo essere fluida a 60 FPS.
	let pipeline_description = r#"
        libcamerasrc !
        video/x-raw,width=1280,height=720,framerate=60/1 !
        queue !
        glupload !
        glcolorconvert !
        gtkglsink name=video_sink
    "#;

	// Crea la pipeline GStreamer
	let pipeline = gst::parse_launch(pipeline_description)
		.expect("Impossibile creare la pipeline GStreamer")
		.downcast::<gst::Pipeline>()
		.expect("L'elemento creato non è una Pipeline");

	// Recupera gtkglsink tramite il nome assegnato sopra
	let video_sink = pipeline
		.by_name("video_sink")
		.expect("Impossibile trovare video_sink");

	// gtkglsink fornisce direttamente il GtkWidget
	let video_widget = video_sink.property::<gtk::Widget>("widget");

	// Finestra GTK
	let window = gtk::Window::new(gtk::WindowType::Toplevel);

	window.set_title("Video Live Delay");
	window.set_default_size(1280, 800);

	// Il video occupa la finestra
	window.add(&video_widget);

	// Chiudendo la finestra terminiamo l'app
	window.connect_destroy(|_| {
		gtk::main_quit();
	});

	// ESC per uscire dal fullscreen durante lo sviluppo
	window.connect_key_press_event(|_, event| {
		if event.keyval() == gtk::gdk::keys::constants::Escape {
			gtk::main_quit();
		}

		glib::Propagation::Proceed
	});

	window.show_all();
	window.fullscreen();

	// Avvia GStreamer
	pipeline
		.set_state(gst::State::Playing)
		.expect("Impossibile avviare la pipeline");

	println!("Pipeline avviata");
	println!("1280x720 @ 60 FPS");
	println!("ESC per uscire");

	// Main loop GTK
	gtk::main();

	// Spegnimento
	println!("Arresto pipeline...");

	pipeline
		.set_state(gst::State::Null)
		.expect("Impossibile arrestare la pipeline");

	println!("Terminato");
}