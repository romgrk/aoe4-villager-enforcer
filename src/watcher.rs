use std::time::Duration;
use std::sync::Arc;
use parking_lot::RwLock;
use image::DynamicImage;
use image::GenericImageView;
use find_subimage::SubImageFinderState;

use crate::State;
use crate::Interface;
use crate::capture;
use crate::sound;

const NOTE: f32 = 12_800.0;

#[derive(Clone)]
pub struct Watcher {
  pub state: Arc<RwLock<State>>,
}

impl Watcher {
  pub fn new(state: Arc<RwLock<State>>) -> Self {
    let watcher = Watcher {
      state: state.clone(),
    };

    let mut watcher_thread = watcher.clone();
    std::thread::spawn(move || {
      loop {
        println!("CHECK");
        watcher_thread.check();

        std::thread::sleep(Duration::from_secs(1));
      }
    });

    return watcher;
  }
  
  fn check(&mut self) {
    println!("check: lock");
    let mut state = self.state.write();

    if state.window_capture.is_none() {
      return;
    }

    println!("check: capture");
    let capture = capture::take_one(state.window_capture.as_ref().unwrap().window.id());
    println!("check: capture: {:?}", capture.is_some());
    if capture.is_none() {
      state.window_capture = None;
      state.interface = Interface::WindowSelect;
      return;
    }
    println!("check: captured");
    println!("check: {}", state.is_watching);

    if state.is_watching == false {
      return;
    }

    let capture = capture.unwrap();

    // Slice the haystack a bit to make it faster
    let x = 0;
    let y = capture.data.height() / 2;
    let width = capture.data.width() / 4;
    let height = state.config.y_max - y;
    let haystack_image = capture.data.view(x, y, width, height);
    let haystack_image = DynamicImage::ImageRgba8(haystack_image.to_image()).to_luma8();

    let needle_image = state.config.data.as_ref().unwrap().clone();

    let mut finder = SubImageFinderState::new();

    println!("check: find");

    // find_subimage_positions() is a long operation
    drop(state);

    println!("check: did_run");

    // These are (x, y, distance) where x and y are the position within the larger image
    // and distance is the distance value, where a smaller distance means a more precise match
    let positions =
      finder.find_subimage_positions(
        (&haystack_image.as_raw(), haystack_image.width() as usize, haystack_image.height() as usize),
        (&needle_image.as_raw(), needle_image.width() as usize, needle_image.height() as usize),
        1
      );

    let position: Option<&(usize, usize, f32)> =
      positions
        .iter()
        .min_by(|(_, _, dist), (_, _, dist2)| dist.partial_cmp(dist2).unwrap());

    println!("FOUND: {:?}", &position);
    println!("POSITIONS: {:?}", positions);

    let mut state = self.state.write();

    state.window_capture = Some(capture);
    state.is_queued = position.is_some();

    if state.is_queued == false {
      println!("check: play_tone");
      sound::play_tone(NOTE, Duration::from_millis(500));
    }
  }
}
