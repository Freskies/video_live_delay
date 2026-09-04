use std::cell::Cell;
use std::process::Command;
use std::rc::Rc;

use gstreamer as gst;
use gstreamer_video as gst_video;
use gtk::prelude::*;
use gtk::GestureLongPress;

use crate::delay::DelayController;

const ROTATIONS: [gst_video::VideoOrientationMethod; 4] = [
	gst_video::VideoOrientationMethod::Identity,
	gst_video::VideoOrientationMethod::_90r,
	gst_video::VideoOrientationMethod::_180,
	gst_video::VideoOrientationMethod::_90l,
];

fn rotate_video(video_sink: &gst::Element, rotation_index: &Cell<usize>) {
	let next = (rotation_index.get() + 1) % ROTATIONS.len();
	rotation_index.set(next);
	let rotation = ROTATIONS[next];
	video_sink.set_property("rotate-method", rotation);
	println!("Rotation: {:?}", rotation);
}

fn refresh_delay_label(label: &gtk::Label, delay: &DelayController) {
	label.set_text(&delay.display_text());
	println!("Selected Delay: {}", delay.display_text());
}

fn install_css() {
	let provider = gtk::CssProvider::new();

	provider
		.load_from_data(include_bytes!("../assets/style.css"))
		.expect("CSS Loading Error");

	let screen = gtk::gdk::Screen::default().expect("No GTK Screen Found");

	gtk::StyleContext::add_provider_for_screen(
		&screen,
		&provider,
		gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
	);
}

pub fn build_window(
	video_widget: gtk::Widget,
	video_sink: gst::Element,
	delay: DelayController,
) -> gtk::Window {
	install_css();

	let rotation_index = Rc::new(Cell::new(0usize));
	let window = gtk::Window::new(gtk::WindowType::Toplevel);
	window.set_title("Video Live Delay");
	window.set_default_size(1280, 800);

	let overlay = gtk::Overlay::new();
	video_widget.set_hexpand(true);
	video_widget.set_vexpand(true);
	overlay.add(&video_widget);

	let controls = gtk::Box::new(gtk::Orientation::Horizontal, 10);
	controls.set_widget_name("controls");
	controls.set_halign(gtk::Align::Center);
	controls.set_valign(gtk::Align::End);
	controls.set_margin_bottom(20);

	let minus_button = gtk::Button::with_label("−");
	minus_button.set_size_request(90, 70);
	controls.pack_start(&minus_button, false, false, 0);

	let plus_button = gtk::Button::with_label("+");
	plus_button.set_size_request(90, 70);
	controls.pack_start(&plus_button, false, false, 0);

	let rotate_button = gtk::Button::with_label("↻");
	rotate_button.set_size_request(90, 70);
	controls.pack_start(&rotate_button, false, false, 0);

	let delay_label = gtk::Label::new(Some(&delay.display_text()));
	delay_label.set_widget_name("delay-label");
	controls.pack_start(&delay_label, false, false, 0);

	let shutdown_button =
		gtk::Button::from_icon_name(Some("system-shutdown-symbolic"), gtk::IconSize::Button);
	shutdown_button.set_widget_name("shutdown-button");
	shutdown_button.set_size_request(90, 70);
	controls.pack_start(&shutdown_button, false, false, 0);

	overlay.add_overlay(&controls);
	window.add(&overlay);

	// - DELAY
	{
		let delay = delay.clone();
		let delay_label = delay_label.clone();

		minus_button.connect_clicked(move |_| {
			delay.decrease();
			refresh_delay_label(&delay_label, &delay);
		});
	}

	// + DELAY
	{
		let delay = delay.clone();
		let delay_label = delay_label.clone();

		plus_button.connect_clicked(move |_| {
			delay.increase();
			refresh_delay_label(&delay_label, &delay);
		});
	}

	// ROTATE
	{
		let video_sink = video_sink.clone();
		let rotation_index = rotation_index.clone();

		rotate_button.connect_clicked(move |_| {
			rotate_video(&video_sink, &rotation_index);
		});
	}

	let shutdown_gesture: GestureLongPress = setup_shutdown_button(&shutdown_button);
	window.connect_destroy(move |_| {
		let _keep_alive = &shutdown_gesture;
		gtk::main_quit();
	});

	window
}

fn setup_shutdown_button(button: &gtk::Button) -> GestureLongPress {
	if let Some(settings) = gtk::Settings::default() {
		settings.set_property("gtk-long-press-time", 1000u32);
	}

	let gesture = GestureLongPress::new(button);
	gesture.set_delay_factor(2.0);

	gesture.connect_pressed(|_, _, _| {
		println!("Shutdown requested");

		match Command::new("systemctl").arg("poweroff").spawn() {
			Ok(_) => {
				println!("Poweroff command sent");
			}
			Err(err) => {
				eprintln!("Unable to shut down: {}", err);
			}
		}
	});

	gesture
}
