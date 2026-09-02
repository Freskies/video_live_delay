use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

use gst::prelude::*;
use gtk::prelude::*;

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

// ------------------------------------------------------------
// CONFIGURAZIONE
// ------------------------------------------------------------

const FPS: usize = 60;
const DELAY_SECONDS: usize = 2;
const DELAY_FRAMES: usize = FPS * DELAY_SECONDS;

const ROTATIONS: [gst_video::VideoOrientationMethod; 4] = [
	gst_video::VideoOrientationMethod::Identity,
	gst_video::VideoOrientationMethod::_90r,
	gst_video::VideoOrientationMethod::_180,
	gst_video::VideoOrientationMethod::_90l,
];

// ------------------------------------------------------------
// ROTAZIONE
// ------------------------------------------------------------

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

// ------------------------------------------------------------
// MAIN
// ------------------------------------------------------------

fn main() {
	gtk::init().expect("Impossibile inizializzare GTK");
	gst::init().expect("Impossibile inizializzare GStreamer");

	println!("Avvio Video Live Delay...");
	println!("Delay: {} secondi", DELAY_SECONDS);
	println!("Buffer: {} frame", DELAY_FRAMES);

	/*
	 * Abbiamo due parti della pipeline.
	 *
	 * CAPTURE:
	 *
	 * camera -> appsink
	 *
	 * PLAYBACK:
	 *
	 * appsrc -> OpenGL -> gtkglsink
	 *
	 * Nel mezzo c'è Rust.
	 */
	let pipeline_description = r#"

        libcamerasrc !
        video/x-raw,width=1280,height=720,framerate=60/1 !
        appsink
            name=capture_sink
            sync=false
            max-buffers=2
            drop=true

        appsrc
            name=playback_src
            is-live=true
            format=time
            do-timestamp=true
        !
        queue
            max-size-buffers=4
            max-size-bytes=0
            max-size-time=0
            leaky=downstream
        !
        glupload !
        glcolorconvert !
        gtkglsink name=video_sink

    "#;

	let pipeline = gst::parse_launch(pipeline_description)
		.expect("Impossibile creare la pipeline")
		.downcast::<gst::Pipeline>()
		.expect("L'elemento creato non è una Pipeline");

	// ---------------------------------------------------------
	// ELEMENTI GSTREAMER
	// ---------------------------------------------------------

	let capture_sink = pipeline
		.by_name("capture_sink")
		.expect("capture_sink non trovato")
		.downcast::<gst_app::AppSink>()
		.expect("capture_sink non è un AppSink");

	let playback_src = pipeline
		.by_name("playback_src")
		.expect("playback_src non trovato")
		.downcast::<gst_app::AppSrc>()
		.expect("playback_src non è un AppSrc");

	let video_sink = pipeline
		.by_name("video_sink")
		.expect("video_sink non trovato");

	let video_widget =
		video_sink.property::<gtk::Widget>("widget");

	// ---------------------------------------------------------
	// BUFFER DELAY
	// ---------------------------------------------------------

	{
		let playback_src = playback_src.clone();

		/*
		 * Questo buffer appartiene al callback.
		 *
		 * Con 2 secondi @ 60 FPS conterrà circa 120 frame.
		 */
		let mut delay_buffer: VecDeque<gst::Buffer> =
			VecDeque::with_capacity(DELAY_FRAMES + 2);

		let mut caps_configured = false;
		let mut playback_started = false;

		capture_sink.set_callbacks(
			gst_app::AppSinkCallbacks::builder()
				.new_sample(move |appsink| {
					// Prende il frame appena arrivato dalla camera.
					let sample = appsink
						.pull_sample()
						.map_err(|_| gst::FlowError::Eos)?;

					// La prima volta copiamo anche il formato video
					// dalla camera verso appsrc.
					if !caps_configured {
						let caps = sample
							.caps_owned()
							.ok_or(gst::FlowError::NotNegotiated)?;

						println!("Formato camera: {}", caps);

						playback_src.set_caps(Some(&caps));

						caps_configured = true;
					}

					let input_buffer = sample
						.buffer()
						.ok_or(gst::FlowError::Error)?;

					/*
					 * IMPORTANTE:
					 *
					 * facciamo una vera copia del frame.
					 *
					 * Non vogliamo tenere occupato il buffer originale
					 * appartenente a libcamerasrc/libcamera.
					 */
					let owned_buffer = input_buffer
						.copy_deep()
						.map_err(|err| {
							eprintln!(
								"Errore copia frame: {}",
								err
							);

							gst::FlowError::Error
						})?;

					// Inserisce il nuovo frame in fondo.
					delay_buffer.push_back(owned_buffer);

					/*
					 * Finché non abbiamo più di 120 frame,
					 * non mostriamo niente.
					 *
					 * Quindi all'avvio lo schermo resta nero
					 * per circa 2 secondi.
					 */
					if delay_buffer.len() <= DELAY_FRAMES {
						return Ok(gst::FlowSuccess::Ok);
					}

					/*
					 * Dopo 2 secondi:
					 *
					 * entra frame 121
					 * esce frame 1
					 *
					 * entra frame 122
					 * esce frame 2
					 *
					 * ecc.
					 */
					let mut output_buffer = delay_buffer
						.pop_front()
						.expect("Delay buffer vuoto");

					/*
					 * Il frame conserva il timestamp originale della
					 * camera.
					 *
					 * Lo cancelliamo perché appsrc, con
					 * do-timestamp=true, gli assegnerà il timestamp
					 * corrente al momento della riproduzione.
					 */
					{
						let buffer = output_buffer
							.get_mut()
							.expect("Buffer non modificabile");

						buffer.set_pts(None::<gst::ClockTime>);
						buffer.set_dts(None::<gst::ClockTime>);
					}

					if !playback_started {
						println!(
							"Buffer pieno: playback iniziato con {} s di delay",
							DELAY_SECONDS
						);

						playback_started = true;
					}

					/*
					 * Reinserisce il vecchio frame nella pipeline
					 * di visualizzazione.
					 */
					if let Err(err) =
						playback_src.push_buffer(output_buffer)
					{
						eprintln!(
							"Errore appsrc push_buffer: {:?}",
							err
						);
					}

					Ok(gst::FlowSuccess::Ok)
				})
				.build(),
		);
	}

	// ---------------------------------------------------------
	// GTK
	// ---------------------------------------------------------

	let rotation_index =
		Rc::new(Cell::new(0usize));

	let window =
		gtk::Window::new(gtk::WindowType::Toplevel);

	window.set_title("Video Live Delay");
	window.set_default_size(1280, 800);

	let overlay = gtk::Overlay::new();

	overlay.add(&video_widget);

	// ---------------------------------------------------------
	// CONTROLLI
	// ---------------------------------------------------------

	let controls =
		gtk::Box::new(gtk::Orientation::Horizontal, 10);

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
	// ROTAZIONE TOUCH
	// ---------------------------------------------------------

	{
		let video_sink = video_sink.clone();
		let rotation_index = rotation_index.clone();

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
		let rotation_index = rotation_index.clone();

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

	println!("Pipeline PLAYING");
	println!("1280x720 @ 60 FPS");
	println!("Delay fisso: {} s", DELAY_SECONDS);
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