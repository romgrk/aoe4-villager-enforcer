mod config;
mod contour;

use std::time::Instant;
use std::time::Duration;
use eframe::egui;
use egui::{Image, ColorImage, Layout, TextureHandle};
use xcap::Window;
use image::DynamicImage;
use image::GenericImageView;
use find_subimage::SubImageFinderState;
use rodio::{OutputStream, Sink};
use rodio::source::{SineWave, Source};

use contour::detect_squares;
use config::Config;

const DEFAULT_WINDOW_TITLE: &str = "Age of Empires IV ";

fn main() -> eframe::Result {
  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
    ..Default::default()
  };

  eframe::run_native(
    "AOE4 Villager Enforcer",
    options,
    Box::new(|cc| {
      egui_extras::install_image_loaders(&cc.egui_ctx);

      cc.egui_ctx.set_pixels_per_point(1.25);

      Ok(Box::<EnforcerApp>::default())
    }),
  )
}

enum Interface {
  WindowSelect,
  RegionSelect,
  Main,
}

struct EnforcerApp {
  interface: Interface,
  capture_index: i64,
  captures: Option<Vec<WindowScreenshot>>,
  last_capture: Instant,
  region_select_state: Option<RegionSelectState>,
  config: Config,
  villager_texture: Option<TextureHandle>,
  is_watching: bool,
  is_queued: bool,
}

struct RegionSelectState {
  display_texture: TextureHandle,
  region_squares: Vec<contour::Square>,
  region_images: Vec<image::RgbaImage>,
  region_textures: Vec<TextureHandle>,
}

impl Default for EnforcerApp {
  fn default() -> EnforcerApp {
    EnforcerApp {
      interface: Interface::WindowSelect,
      capture_index: -1,
      captures: None,
      last_capture: Instant::now(),
      region_select_state: None,
      config: config::load(),
      villager_texture: None,
      is_watching: false,
      is_queued: false,
    }
  }
}

impl eframe::App for EnforcerApp {
  fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    if self.captures.is_none() {
      self.captures = Some(take_screenshots());
    }

    match self.interface {
      Interface::WindowSelect => {
        self.ui_window_select(ctx, frame);
      }
      Interface::RegionSelect => {
        self.ui_region_select(ctx, frame);
      }
      Interface::Main => {
        self.ui_main(ctx, frame);
      }
    }
  }
}

impl EnforcerApp {
  fn ui_window_select(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let captures = self.captures.as_mut().unwrap();

    egui::CentralPanel::default().show(ctx, |ui| {
      egui::ScrollArea::both().show(ui, |ui| {
        ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
          let mut index = 0;
          captures.iter_mut().for_each(|capture| {
            ui.vertical(|ui| {
              let texture = capture.texture.get_or_insert_with(|| {
                ui.ctx().load_texture(
                  capture.title.clone(),
                  image_to_egui(&capture.data),
                  Default::default(),
                )
              });

              let button = egui::Button::image_and_text(
                Image::from_texture((texture.id(), texture.size_vec2()))
                  .max_height(200.0),
                &capture.title,
              );

              if ui.add(button).clicked() {
                self.capture_index = index;
                self.interface = Interface::RegionSelect;
              }
            });
            index += 1;
          });
        });
      });
    });
  }

  fn ui_region_select(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let captures = self.captures.as_mut().unwrap();
    let capture_index = self.capture_index as usize;
    let capture = captures.get_mut(capture_index).unwrap();

    let state = self.region_select_state.get_or_insert_with(|| {
      let source_image = &capture.data;

      let processing = source_image;
      let processing = image::imageops::colorops::grayscale(processing);
      let grayscale_image = processing.clone();
      let processing = {
        let n = 120;
        imageproc::contrast::stretch_contrast(&processing, n, n + 1, 0, 255)
      };

      let processed_image = processing;

      let contours = imageproc::contours::find_contours::<i32>(&processed_image);

      let squares = detect_squares(
        processed_image.width(),
        processed_image.height(),
        &contours
      );

      let mut image = DynamicImage::ImageLuma8(grayscale_image).to_rgba8();
      // let mut image = DynamicImage::ImageLuma8(processed_image).to_rgba8();
      // let mut image = source_image.clone();

      let mut region_images = vec![];
      let mut region_textures = vec![];

      for square in squares.iter() {
        for point in &square.contour.points {
          image.put_pixel(
            point.x as u32,
            point.y as u32,
            image::Rgba([255, 0, 0, 255])
          );
        }

        let offset = 5;
        let region_image = source_image.view(
          (square.points[0].x() as u32) + offset,
          (square.points[0].y() as u32) + offset,
          (square.points[2].x() as u32 - square.points[0].x() as u32) - offset * 2,
          (square.points[2].y() as u32 - square.points[0].y() as u32) - offset * 2,
        ).to_image();

        region_images.push(region_image.clone());
        region_textures.push(ctx.load_texture(
          format!("{}-contoured", capture.title.clone()),
          image_to_egui(&region_image),
          Default::default(),
        ));
      }

      let display_texture = ctx.load_texture(
        format!("{}-contoured", capture.title.clone()),
        image_to_egui(&image),
        Default::default(),
      );

      RegionSelectState {
        display_texture,
        region_squares: squares,
        region_images,
        region_textures,
      }
    });

    egui::CentralPanel::default().show(ctx, |ui| {
      egui::ScrollArea::both().show(ui, |ui| {

        ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
          ui.vertical(|ui| {
            ui.horizontal(|ui| {
              state.region_textures.iter().enumerate().for_each(|(index, texture)| {
                ui.vertical(|ui| {

                  let button = egui::Button::image_and_text(
                    Image::from_texture((texture.id(), texture.size_vec2()))
                      .max_height(80.0),
                    "Select",
                  );

                  if ui.add(button).clicked() {

                    // Select half the image
                    let image = &state.region_images[index];
                    let image = image.view(
                      image.width() / 2,
                      10,
                      image.width() / 2,
                      image.height() / 2,
                    );
                    let image = image.to_image();
                    let image = DynamicImage::ImageRgba8(image).to_luma8();

                    self.config.data = Some(image);
                    self.config.y_max = state.region_squares[index].points[3].y() as u32;

                    config::write(&self.config);

                    println!("Config: {:?}", self.config);

                    self.interface = Interface::Main;
                  }
                });
              });
            });

            ui.add(
              Image::from_texture((
                state.display_texture.id(),
                state.display_texture.size_vec2()
              ))
                .max_height(600.0)
            );
          });
        });
      });
    });
  }

  fn ui_main(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let captures = self.captures.as_mut().unwrap();
    let capture_index = self.capture_index as usize;

    if self.is_watching && Instant::now().duration_since(self.last_capture) > Duration::from_secs(1) {
      let capture = take_screenshot(capture_index);

      // Slice the haystack a bit to make it faster
      let x = 0;
      let y = capture.data.height() / 2;
      let width = capture.data.width() / 4;
      let height = self.config.y_max - y;
      let haystack_image = capture.data.view(x, y, width, height);
      let haystack_image = DynamicImage::ImageRgba8(haystack_image.to_image()).to_luma8();

      let needle_image = &self.config.data.as_ref().unwrap();

      let mut finder = SubImageFinderState::new();

      println!("RUNNING FIND");

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

      captures[capture_index] = capture;

      self.is_queued = position.is_some();

      if self.is_queued == false {
        std::thread::spawn(|| {
          play_tone();
        });
      }
    }

    let capture = captures.get_mut(capture_index).unwrap();
    let capture_texture = capture.texture.get_or_insert_with(|| {
      ctx.load_texture(
        capture.title.clone(),
        image_to_egui(&capture.data),
        Default::default(),
      )
    });

    let villager_texture = self.villager_texture.get_or_insert_with(|| {
      ctx.load_texture(
        "villager-texture",
        image_to_egui(&DynamicImage::ImageLuma8(self.config.data.as_ref().unwrap().clone()).to_rgba8()),
        Default::default(),
      )
    });

    ctx.request_repaint_after_secs(1.0);

    egui::CentralPanel::default().show(ctx, |ui| {
      egui::ScrollArea::both().show(ui, |ui| {

        ui.vertical(|ui| {
          ui.horizontal(|ui| {
            ui.label("Status: ");
            let text = if self.is_watching { "Running" } else { "Not running" };
            let color = if self.is_watching {
              egui::Color32::from_rgb(10, 225, 70)
            } else {
              ui.visuals().text_color()
            };
            ui.label(egui::RichText::new(text).color(color))
          });

          ui.horizontal(|ui| {
            ui.label("Villager queued?:");
            let text = if self.is_watching && self.is_queued { "Yes" } else { "No" };
            let color = if !self.is_watching {
              ui.visuals().text_color()
            } else if self.is_queued {
              egui::Color32::from_rgb(10, 225, 70)
            } else {
              egui::Color32::from_rgb(225, 10, 50)
            };
            ui.label(egui::RichText::new(text).color(color))
          });

          if ui.button(if self.is_watching { "Stop" } else { "Start" }).clicked() {
            self.is_watching = !self.is_watching;
          }

          ui.add(
            Image::from_texture((
              villager_texture.id(),
              villager_texture.size_vec2()
            ))
              .max_height(100.0)
          );
          ui.add(
            Image::from_texture((
              capture_texture.id(),
              capture_texture.size_vec2()
            ))
              .max_height(400.0)
          );
        });
      });
    });
  }
}

struct WindowScreenshot {
  title: String,
  data: image::RgbaImage,
  texture: Option<TextureHandle>,
}

fn take_screenshots() -> Vec<WindowScreenshot> {
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

    let title = window.title();
    let image = window.capture_image().unwrap();

    let result = WindowScreenshot {
      title: if title == ""  { "(empty)".into() } else { title.into() },
      data: image,
      texture: None,
    };

    results.push(result);
  }

  return results;
}

fn take_screenshot(window_index: usize) -> WindowScreenshot {
  let windows = Window::all().unwrap();
  let window = &windows[window_index];

  println!(
    "Window: {:?} {:?}",
    window.title(),
    (window.x(), window.y(), window.width(), window.height()),
  );

  let title = window.title();
  let image = window.capture_image().unwrap();

  return WindowScreenshot {
    title: if title == ""  { "(empty)".into() } else { title.into() },
    data: image,
    texture: None,
  };
}

fn play_tone() {
  // _stream must live as long as the sink
  let (_stream, stream_handle) = OutputStream::try_default().unwrap();
  let sink = Sink::try_new(&stream_handle).unwrap();

  sink.append(
    SineWave::new(12800.0)
      .take_duration(Duration::from_secs_f32(0.25))
      .amplify(4.00)
  );

  // The sound plays in a separate thread. This call will block the current thread until the sink
  // has finished playing all its queued sounds.
  sink.sleep_until_end();
}

fn image_to_egui(image: &image::RgbaImage) -> egui::ColorImage {
  ColorImage::from_rgba_unmultiplied(
    [image.width() as usize, image.height() as usize],
    image.as_raw()
  )
}

