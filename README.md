# Video Live Delay

A portable, open-source delayed video feedback system built for outdoor training.

Video Live Delay is designed to run on a Raspberry Pi connected to a camera and a touchscreen display. Place the device
near your training area, set a delay, perform a movement, and then look at the screen to watch yourself a few seconds
later without touching a phone or manually replaying a recording.

Instead of recording a video, stopping the session, opening the clip, rewinding it, and starting again, the system
continuously shows the camera feed with a configurable delay.

## What it does

The camera captures continuously at 60 FPS while the application keeps a short history of recent frames in memory.

The video is not compressed because I am a garbage programmer and didn't know how to do it. If you're able you can
contribute to the project! Otherwise, I suggest sticking to at least 4GB of RAM.

## Features

(Written by AI but (not carefully) reviewed by me)

- Live camera capture at **1280×720 @ 60 FPS**
- Configurable video delay
- Delay shown directly in the interface
- Touchscreen controls to increase or decrease the delay
- GPU-accelerated video rendering
- GPU-accelerated 0° / 90° / 180° / 270° rotation
- Fullscreen GTK interface
- Designed to run locally without an Internet connection
- Written in **Rust**
- Built on **GStreamer**, **GTK 3**, **libcamera**, and **OpenGL**
- Open source under the **GNU GPL v3 or later**

The current design targets delays up to approximately **15 seconds** on a Raspberry Pi with 4 GB of RAM.

## Hardware used for development

This project is being developed and tested with the following hardware:

- **Raspberry Pi 4 Model B**
    - 4 GB RAM
- **Raspberry Pi Camera Module 3**
    - Sony IMX708 sensor
- **10.1-inch Wondershare touchscreen display**
    - 1280×800 display resolution
- microSD card with Raspberry Pi OS (64GB, but it's definitely too much)
- Power Bank INIU 20000mAh 45W

### RAM requirements

The delayed video buffer currently stores raw frames in memory.

At 1280×720 and 60 FPS, the memory usage increases with the configured maximum delay.

As a practical guideline:

- **2 GB Raspberry Pi:** recommended for shorter delays, roughly up to 8–10 seconds
- **4 GB Raspberry Pi:** recommended for delays up to approximately 15–20 seconds

The default project configuration is intended for a **4 GB Raspberry Pi** with a maximum delay of **15 seconds**.

So if you want to use a Raspberry with 2GB of RAM be sure to change the `MAX_DELAY_FRAMES` in `config.rs` from
`FPS * 15` to `FPS * 8`.

## Software environment

The development system uses:

- Raspberry Pi OS
- Wayland
- labwc compositor
- libcamera
- GStreamer 1.26.x
- GTK 3
- Rust / Cargo

The video pipeline uses the Raspberry Pi camera through `libcamerasrc` and renders through OpenGL.

The working rendering path is conceptually (ty sig. GPT for this exhaustive schema):

```text
Camera
  ↓
libcamerasrc
  ↓
1280×720 @ 60 FPS
  ↓
Rust video buffer
  ↓
appsrc
  ↓
glupload
  ↓
glcolorconvert
  ↓
gtkglsink
  ↓
GTK fullscreen interface
```

Rotation is performed directly by `gtkglsink`, avoiding CPU-heavy frame rotation.

### Main modules

- `main.rs` — application startup and shutdown
- `config.rs` — resolution, frame rate, delay limits, and other configuration
- `delay.rs` — current delay state and delay controls
- `pipelines.rs` — GStreamer pipeline definitions
- `ui.rs` — GTK window, touchscreen controls, label, CSS, and rotation controls
- `video.rs` — capture/playback pipeline management
- `video_buffer.rs` — frame history and delayed-frame selection
- `assets/style.css` — GTK interface styling

## Requirements

You need a working Raspberry Pi camera setup and the required GTK/GStreamer development libraries.

Before building the Rust application, verify that the required GStreamer elements are available:

```bash
gst-inspect-1.0 libcamerasrc
gst-inspect-1.0 glupload
gst-inspect-1.0 glcolorconvert
gst-inspect-1.0 gtkglsink
```

All four commands should return information about the corresponding GStreamer element.

You should also verify that the camera works at the target resolution and frame rate:

```bash
gst-launch-1.0 libcamerasrc ! \
'video/x-raw,width=1280,height=720,framerate=60/1' ! \
queue ! \
glupload ! \
glcolorconvert ! \
gtkglsink
```

If this pipeline is smooth, the Raspberry Pi video path is working correctly.

## Rust dependencies

The project currently uses:

```toml
[dependencies]
gtk = "0.18"
gstreamer = "0.21"
gstreamer-video = "0.21"
gstreamer-app = "0.21"
```

These versions are intentionally kept on compatible GTK/GLib/GStreamer Rust binding generations.

## Build

Clone the repository and enter the project directory:

```bash
git clone <repository-url>
cd video_live_delay
```

Build the optimized release version:

```bash
cargo build --release
```

The executable will be created at:

```text
target/release/video_live_delay
```

## Run

Start the application with:

```bash
./target/release/video_live_delay
```

The application opens fullscreen and starts the camera automatically.

Use the touchscreen controls to:

- decrease the video delay
- see the currently selected delay
- increase the video delay
- rotate the video

The interface is designed to be usable without a keyboard.

## Configuration

The main video settings are kept in `src/config.rs`.

Typical values include:

```rust
pub const VIDEO_WIDTH: usize = 1280;
pub const VIDEO_HEIGHT: usize = 720;
pub const FPS: usize = 60;
```

The maximum delay is also configured there.

For example, a 15-second maximum at 60 FPS corresponds to:

```rust
pub const MAX_DELAY_FRAMES: usize = FPS * 15;
```

Reducing this value lowers the maximum RAM used by the video history.

## Autostart

The application is intended to run automatically when the Raspberry Pi boots, turning the device into a dedicated
training tool rather than a general-purpose computer.

A production installation can launch:

```text
target/release/video_live_delay
```

from the Raspberry Pi desktop/session startup or a suitable system service.

The exact autostart configuration can depend on the Raspberry Pi OS desktop and Wayland session, so it is best
configured after the application is confirmed to run correctly in the graphical session.

## Why Rust?

Because I fucking love Rust, and you'd better start too!

## Contributing

Contributions, fixes, experiments, and improvements are welcome.

If you modify and distribute the project, the GNU GPL requires the distributed derivative work and corresponding source
code to remain available under the same license terms.

## License

Copyright (C) 2026 Valerio Giacchini

This project is free and open-source software licensed under the **GNU General Public License v3.0 or later**.

See the [LICENSE](LICENSE) file for the complete license text.