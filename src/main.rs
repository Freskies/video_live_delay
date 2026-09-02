use gstreamer as gst;

use gst::prelude::*;
use gtk::prelude::*;

fn main() {
	// Inizializza GTK
	gtk::init().expect("Impossibile inizializzare GTK");

	// Inizializza GStreamer
	gst::init().expect("Impossibile inizializzare GStreamer");

	println!("Avvio Video Live Delay...");

	// Pipeline che abbiamo già verificato sul Raspberry Pi.
	let pipeline_description = r#"
        libcamerasrc !
        video/x-raw,width=1280,height=720,framerate=60/1 !
        queue !
        glupload !
        glcolorconvert !
        gtkglsink name=video_sink
    "#;

	let pipeline = gst::parse::launch(pipeline_description)
		.expect("Impossibile creare la pipeline GStreamer")
		.downcast::<gst::Pipeline>()
		.expect("L'elemento creato non è una pipeline");

	// Recupera gtkglsink.
	let video_sink = pipeline
		.by_name("video_sink")
		.expect("gtkglsink non trovato");

	// gtkglsink crea direttamente un GtkWidget.
	let video_widget = video_sink.property::<gtk::Widget>("widget");

	// Finestra principale.
	let window = gtk::Window::new(gtk::WindowType::Toplevel);

	window.set_title("Video Live Delay");
	window.set_default_size(1280, 800);

	// Il widget video occupa tutta la finestra.
	window.add(&video_widget);

	// Chiudendo la finestra terminiamo GTK.
	window.connect_delete_event(|_, _| {
		gtk::main_quit();
		gtk::Inhibit(false)
	});

	// ESC permette di uscire durante lo sviluppo.
	window.connect_key_press_event(|_, event| {
		if event.keyval() == gtk::gdk::keys::constants::Escape {
			gtk::main_quit();
		}

		gtk::Inhibit(false)
	});

	window.show_all();

	// Fullscreen dopo aver creato e mostrato la finestra.
	window.fullscreen();

	// Avvia la camera.
	pipeline
		.set_state(gst::State::Playing)
		.expect("Impossibile avviare la pipeline");

	println!("Pipeline avviata.");
	println!("1280x720 @ 60 FPS");
	println!("Premi ESC per uscire.");

	// Loop principale GTK.
	gtk::main();

	println!("Arresto pipeline...");

	// Spegne correttamente camera e pipeline.
	pipeline
		.set_state(gst::State::Null)
		.expect("Impossibile arrestare la pipeline");

	println!("Terminato.");
}
