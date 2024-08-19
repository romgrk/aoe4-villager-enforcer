use xcap::Window;
use egui::TextureHandle;

#[derive(Clone)]
pub struct Capture {
  pub window: Window,
  pub data: image::RgbaImage,
  pub texture: Option<TextureHandle>,
}

pub fn take_all() -> Vec<Capture> {
  let windows = Window::all().unwrap();
  let mut results = vec![];

  for window in windows {
    println!(
      "Window: {:?} {:?} {:?} {:?}",
      window.id(),
      window.app_name(),
      window.title(),
      (window.x(), window.y(), window.width(), window.height()),
    );

    let image = window.capture_image().unwrap();

    let result = Capture {
      window,
      data: image,
      texture: None,
    };

    results.push(result);
  }

  return results;
}

pub fn take_one(window_id: u32) -> Option<Capture> {
  println!("take_one: start");
  let windows = Window::all().unwrap();
  println!("take_one: windows");

  for window in windows {
    println!(
      "Window: {:?} {:?} {:?} {:?}",
      window.id(),
      window.app_name(),
      window.title(),
      (window.x(), window.y(), window.width(), window.height()),
    );

    if window.id() != window_id {
      continue;
    }

    let image = window.capture_image().unwrap();

    return Some(Capture {
      window,
      data: image,
      texture: None,
    });
  }

  return None;
}

