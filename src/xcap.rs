use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use std::time::Instant;
use rodio::{Decoder, OutputStream, Sink};
use rodio::source::{SineWave, Source};
use xcap::Monitor;

fn normalized(filename: &str) -> String {
    filename
        .replace("|", "")
        .replace("\\", "")
        .replace(":", "")
        .replace("/", "")
}

fn main() {
    let start = Instant::now();
    let monitors = Monitor::all().unwrap();

    for monitor in monitors {
        let image = monitor.capture_image().unwrap();

        image
            .save(format!("monitor-{}.png", normalized(monitor.name())))
            .unwrap();
    }

    println!("运行耗时: {:?}", start.elapsed());

    play_tone();
}

fn play_tone() {
    // _stream must live as long as the sink
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Add a dummy source of the sake of the example.
    let source = SineWave::new(440.0).take_duration(Duration::from_secs_f32(0.25)).amplify(0.20);
    sink.append(source);

    // The sound plays in a separate thread. This call will block the current thread until the sink
    // has finished playing all its queued sounds.
    sink.sleep_until_end();
}
