use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

use gst::prelude::*;
use gtk::prelude::*;

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

// ============================================================
// CONFIG
// ============================================================

const FPS: usize = 60;
const DELAY_SECONDS: usize = 2;
const DELAY_FRAMES: usize = FPS * DELAY_SECONDS;

const ROTATIONS: [gst_video::VideoOrientationMethod; 4] = [
	gst_video::VideoOrientationMethod::Identity,
	gst_video::VideoOrientationMethod::_90r,
	gst_video::VideoOrientationMethod::_180,
	gst_video::VideoOrientationMethod::_90l,
];

// ============================================================
// ROTAZIONE
// ============================================================

fn rotate_video(video_sink: &gst::Element, rotation_index: &Cell<usize>) {
	let next = (rotation_index.get() + 1) % ROTATIONS.len();

	rotation_index.set(next);

	let rotation = ROTATIONS[next];

	video_sink.set_property("rotate-method", rotation);

	println!("Rotazione: {:?}", rotation);
}

// ============================================================
// MAIN
// ============================================================

fn main() {
	gtk::init().expect("Impossibile inizializzare GTK");
	gst::init().expect("Impossibile inizializzare GStreamer");

	println!("Avvio Video Live Delay...");
	println!("Delay: {} secondi", DELAY_SECONDS);
	println!("Buffer: {} frame", DELAY_FRAMES);

	// ========================================================
	// PIPELINE 1: CAMERA
	// ========================================================

	let capture_pipeline = gst::parse_launch(
		r#"
        libcamerasrc !
        video/x-raw,width=1280,height=720,framerate=60/1 !
        appsink
            name=capture_sink
            sync=false
            max-buffers=2
            drop=true
        "#,
	)
		.expect("Impossibile creare capture pipeline")
		.downcast::<gst::Pipeline>()
		.expect("capture_pipeline non è una Pipeline");

	let capture_sink = capture_pipeline
		.by_name("capture_sink")
		.expect("capture_sink non trovato")
		.downcast::<gst_app::AppSink>()
		.expect("capture_sink non è AppSink");

	// ========================================================
	// PIPELINE 2: DISPLAY
	// ========================================================

	let playback_pipeline = gst::parse_launch(
		r#"
        appsrc
            name=playback_src
            is-live=true
            format=time
            do-timestamp=true
            block=false
        !
        queue
            max-size-buffers=4
            max-size-bytes=0
            max-size-time=0
            leaky=downstream
        !
        glupload !
        glcolorconvert !
        gtkglsink
            name=video_sink
    "#,
	)
		.expect("Impossibile creare playback pipeline")
		.downcast::<gst::Pipeline>()
		.expect("playback_pipeline non è una Pipeline");

	let playback_src = playback_pipeline
		.by_name("playback_src")
		.expect("playback_src non trovato")
		.downcast::<gst_app::AppSrc>()
		.expect("playback_src non è AppSrc");

	let video_sink = playback_pipeline
		.by_name("video_sink")
		.expect("video_sink non trovato");

	let video_widget = video_sink.property::<gtk::Widget>("widget");

	// ========================================================
	// DELAY BUFFER
	// ========================================================

	{
		let playback_src = playback_src.clone();

		let mut delay_buffer: VecDeque<gst::Sample> = VecDeque::with_capacity(DELAY_FRAMES + 2);

		let mut frame_counter: u64 = 0;
		let mut playback_started = false;

		capture_sink.set_callbacks(
			gst_app::AppSinkCallbacks::builder()
				.new_sample(move |appsink| {
					// ----------------------------------------
					// Riceve il frame dalla camera
					// ----------------------------------------

					let input_sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;

					frame_counter += 1;

					// Debug: ci permette di sapere immediatamente
					// se appsink sta ricevendo frame.
					if frame_counter == 1 {
						println!("PRIMO FRAME RICEVUTO!");

						if let Some(caps) = input_sample.caps() {
							println!("Caps camera: {}", caps);
						}
					}

					if frame_counter % 60 == 0 {
						println!(
							"Frame camera ricevuti: {} | buffer: {}",
							frame_counter,
							delay_buffer.len()
						);
					}

					let input_buffer = input_sample.buffer().ok_or(gst::FlowError::Error)?;

					let caps = input_sample
						.caps()
						.ok_or(gst::FlowError::NotNegotiated)?
						.to_owned();

					let mut copied_buffer = input_buffer.copy_deep().map_err(|err| {
						eprintln!("Errore copy_deep: {}", err);
						gst::FlowError::Error
					})?;

					{
						let buffer = copied_buffer
							.get_mut()
							.expect("Buffer copiato non modificabile");

						buffer.set_pts(None::<gst::ClockTime>);
						buffer.set_dts(None::<gst::ClockTime>);
					}

					let stored_sample = gst::Sample::builder()
						.buffer(&copied_buffer)
						.caps(&caps)
						.build();

					delay_buffer.push_back(stored_sample);

					// ----------------------------------------
					// RIEMPI IL BUFFER
					// ----------------------------------------

					if delay_buffer.len() <= DELAY_FRAMES {
						return Ok(gst::FlowSuccess::Ok);
					}

					// ----------------------------------------
					// FRAME DI 2 SECONDI FA
					// ----------------------------------------

					let output_sample = delay_buffer.pop_front().expect("Delay buffer vuoto");

					if !playback_started {
						println!();
						println!("==============================");
						println!("BUFFER PIENO");
						println!("Playback avviato con {} secondi di delay", DELAY_SECONDS);
						println!("==============================");
						println!();

						playback_started = true;
					}

					// AppSrc::push_sample trasferisce anche i caps.
					playback_src.push_sample(&output_sample).map_err(|err| {
						eprintln!("Errore push_sample: {:?}", err);

						err
					})?;

					Ok(gst::FlowSuccess::Ok)
				})
				.build(),
		);
	}

	// ========================================================
	// INTERFACCIA GTK
	// ========================================================

	let rotation_index = Rc::new(Cell::new(0usize));

	let window = gtk::Window::new(gtk::WindowType::Toplevel);

	window.set_title("Video Live Delay");
	window.set_default_size(1280, 800);

	let overlay = gtk::Overlay::new();

	overlay.add(&video_widget);

	let controls = gtk::Box::new(gtk::Orientation::Horizontal, 10);

	controls.set_halign(gtk::Align::End);
	controls.set_valign(gtk::Align::End);

	controls.set_margin_end(20);
	controls.set_margin_bottom(20);

	let rotate_button = gtk::Button::with_label("↻");

	rotate_button.set_size_request(90, 70);

	controls.pack_start(&rotate_button, false, false, 0);

	overlay.add_overlay(&controls);

	window.add(&overlay);

	// ========================================================
	// ROTAZIONE
	// ========================================================

	{
		let video_sink = video_sink.clone();
		let rotation_index = rotation_index.clone();

		rotate_button.connect_clicked(move |_| {
			rotate_video(&video_sink, &rotation_index);
		});
	}

	// ========================================================
	// TASTIERA
	// ========================================================

	{
		let video_sink = video_sink.clone();
		let rotation_index = rotation_index.clone();

		window.connect_key_press_event(move |_, event| {
			let key = event.keyval();

			if key == gtk::gdk::keys::constants::Escape {
				gtk::main_quit();
			}

			if key == gtk::gdk::keys::constants::r || key == gtk::gdk::keys::constants::R {
				rotate_video(&video_sink, &rotation_index);
			}

			glib::Propagation::Proceed
		});
	}

	window.connect_destroy(|_| {
		gtk::main_quit();
	});

	// ========================================================
	// MOSTRA FINESTRA
	// ========================================================

	window.show_all();
	window.fullscreen();

	// ========================================================
	// AVVIO PIPELINE
	// ========================================================

	/*
	 * Prima avviamo il display.
	 *
	 * Rimarrà semplicemente in attesa del primo sample proveniente
	 * da appsrc.
	 */
	playback_pipeline
		.set_state(gst::State::Playing)
		.expect("Impossibile avviare playback pipeline");

	println!("Playback pipeline avviata");

	/*
	 * Poi avviamo la camera.
	 */
	capture_pipeline
		.set_state(gst::State::Playing)
		.expect("Impossibile avviare capture pipeline");

	println!("Capture pipeline avviata");
	println!("In attesa di {} frame...", DELAY_FRAMES);

	gtk::main();

	// ========================================================
	// ARRESTO
	// ========================================================

	println!("Arresto camera...");

	capture_pipeline
		.set_state(gst::State::Null)
		.expect("Errore arresto capture pipeline");

	println!("Arresto display...");

	playback_pipeline
		.set_state(gst::State::Null)
		.expect("Errore arresto playback pipeline");

	println!("Terminato");
}
