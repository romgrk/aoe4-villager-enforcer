use std::time::Duration;

use tinyaudio::prelude::*;

pub fn play_tone(note: f32, duration: Duration) {
  std::thread::spawn(move || {
    let params = OutputDeviceParameters {
      channels_count: 2,
      sample_rate: 44100,
      channel_sample_count: 4410,
    };

    let device =
      run_output_device(params, {
        let mut clock = 0f32;
        move |data| {
          for samples in data.chunks_mut(params.channels_count) {
            clock = (clock + 1.0) % params.sample_rate as f32;
            let value =
              (clock * note * 2.0 * std::f32::consts::PI / params.sample_rate as f32).sin();
            for sample in samples {
              *sample = value;
            }
          }
        }
      }).unwrap();

    std::thread::sleep(duration);

    drop(device);
  });
}

