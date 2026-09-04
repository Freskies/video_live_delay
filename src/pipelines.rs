use crate::config::{FPS, VIDEO_HEIGHT, VIDEO_WIDTH};

pub fn capture() -> String {
	format!(
		r#"
		libcamerasrc !
			video/x-raw,
				width={},
				height={},
				framerate={}/1 !
			appsink
				name=capture_sink
				sync=false
				max-buffers=2
				drop=true
		"#,
		VIDEO_WIDTH, VIDEO_HEIGHT, FPS,
	)
}

pub const PLAYBACK: &str = r#"
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
	glupload
	!
	glcolorconvert
	!
	gtkglsink
		name=video_sink
"#;
