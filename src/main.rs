mod config;
mod contour;
mod capture;
mod state;
mod sound;
mod watcher;

use std::sync::Arc;
use std::time::Instant;
use std::time::Duration;
use parking_lot::RwLock;
use eframe::egui;
use egui::{Image, ColorImage, Layout, TextureHandle};
use image::DynamicImage;
use image::GenericImageView;

use contour::detect_squares;
use state::State;
use state::Interface;
use watcher::Watcher;

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

struct EnforcerApp {
  state: Arc<RwLock<State>>,
  region_select_state: Option<RegionSelectState>,
  watcher: Watcher,
}

struct RegionSelectState {
  display_texture: TextureHandle,
  region_squares: Vec<contour::Square>,
  region_images: Vec<image::RgbaImage>,
  region_textures: Vec<TextureHandle>,
}

impl Default for EnforcerApp {
  fn default() -> EnforcerApp {
    let state = Arc::new(RwLock::new(State {
      interface: Interface::WindowSelect,
      captures: None,
      window_capture: None,
      last_capture: Instant::now(),
      config: config::load(),
      villager_texture: None,
      is_watching: false,
      is_queued: false,
    }));

    EnforcerApp {
      watcher: Watcher::new(state.clone()),
      state,
      region_select_state: None,
    }
  }
}

impl eframe::App for EnforcerApp {
  fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    match self.get(|s| s.interface) {
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
  fn get<T>(&self, callback: fn(&State) -> T) -> T {
    let state = self.state.read();
    let value = callback(&state);
    return value;
  }

  fn update(&self, callback: impl Fn(&mut State)) {
    let mut state = self.state.write();
    callback(&mut state);
  }

  fn ui_window_select(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let mut state = self.state.write();

    if state.captures.is_none() || Instant::now().duration_since(state.last_capture) > Duration::from_secs(1) {
      state.captures = Some(capture::take_all());
      state.last_capture = Instant::now();
    }

    let title = state.config.window_title.to_owned();
    let capture = state.captures.as_ref().map(|captures| captures.iter().find(|c| c.window.title() == title)).unwrap();
    if capture.is_some() {
      state.window_capture = capture.cloned();
      state.interface = Interface::RegionSelect;
      drop(state);
      return self.ui_region_select(ctx, _frame);
    }

    egui::CentralPanel::default().show(ctx, |ui| {
      ui.vertical(|ui| {
        ui.heading("Select AOE4 window");

        egui::ScrollArea::both().show(ui, |ui| {
          ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
            let mut index = 0;

            state.captures.as_mut().unwrap().iter_mut().for_each(|capture| {
              capture.texture.get_or_insert_with(|| {
                ui.ctx().load_texture(
                  capture.window.title(),
                  image_to_egui(&capture.data),
                  Default::default(),
                )
              });
            });

            state.captures.as_ref().unwrap().clone().iter().for_each(|capture| {
              ui.vertical(|ui| {
                let texture = capture.texture.as_ref().unwrap();

                let button = egui::Button::image_and_text(
                  Image::from_texture((texture.id(), texture.size_vec2()))
                    .max_height(200.0),
                  capture.window.title(),
                );

                if ui.add(button).clicked() {
                  state.window_capture = Some(capture.clone());
                  state.interface = Interface::RegionSelect;
                }
              });
              index += 1;
            });
          });
        });
      })
    });
  }

  fn ui_region_select(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let mut state = self.state.write();

    if state.config.data.is_some() {
      state.interface = Interface::Main;
      drop(state);
      return self.ui_main(ctx, _frame);
    }

    let capture = state.window_capture.as_ref().unwrap();

    let region_state = self.region_select_state.get_or_insert_with(|| {
      let source_image = &capture.data;

      let processing = source_image;
      let processing = image::imageops::colorops::grayscale(processing);
      let grayscale_image = processing.clone();
      let processing = imageproc::contrast::stretch_contrast(&processing, 75, 90, 0, 255);

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

      for c in contours {
        for point in &c.points {
          image.put_pixel(
            point.x as u32,
            point.y as u32,
            image::Rgba([5, 255, 0, 255])
          );
        }
      }
      for (i, square) in squares.iter().enumerate() {
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
          format!("{}-square-{}", capture.window.title(), i),
          image_to_egui(&region_image),
          Default::default(),
        ));
      }

      let display_texture = ctx.load_texture(
        format!("{}-contoured", capture.window.title()),
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
      ui.vertical(|ui| {
        ui.heading("Select villager image");

        egui::ScrollArea::both().show(ui, |ui| {
          ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
            ui.vertical(|ui| {
              ui.horizontal(|ui| {
                region_state.region_textures.iter().enumerate().for_each(|(index, texture)| {
                  ui.vertical(|ui| {

                    let button = egui::Button::image_and_text(
                      Image::from_texture((texture.id(), texture.size_vec2()))
                        .max_height(80.0),
                      "Select",
                    );

                    if ui.add(button).clicked() {

                      // Select half the image
                      let image = &region_state.region_images[index];
                      let image = image.view(
                        image.width() / 2,
                        10,
                        image.width() / 2,
                        image.height() / 2,
                      );
                      let image = image.to_image();
                      let image = DynamicImage::ImageRgba8(image).to_luma8();

                      state.config.data = Some(image);
                      state.config.y_max = region_state.region_squares[index].points[3].y() as u32 + 20;

                      // XXX: show error message?
                      let _ = config::write(&state.config);

                      println!("Config: {:?}", state.config);

                      state.interface = Interface::Main;
                    }
                  });
                });
              });

              ui.add(
                Image::from_texture((
                  region_state.display_texture.id(),
                  region_state.display_texture.size_vec2()
                ))
                  .max_height(600.0)
              );
            });
          });
        });
      });
    });
  }

  fn ui_main(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let mut state = self.state.write();

    let capture = state.window_capture.as_mut().unwrap();
    let capture_texture = capture.texture.get_or_insert_with(|| {
      ctx.load_texture(
        capture.window.title(),
        image_to_egui(&capture.data),
        Default::default(),
      )
    }).clone();

    let villager_image = state.config.data.as_ref().unwrap().clone();
    let villager_texture = state.villager_texture.get_or_insert_with(|| {
      ctx.load_texture(
        "villager-texture",
        image_to_egui(&DynamicImage::ImageLuma8(villager_image).to_rgba8()),
        Default::default(),
      )
    }).clone();

    egui::CentralPanel::default().show(ctx, |ui| {
      ui.vertical(|ui| {
        ui.horizontal(|ui| {
          ui.label("Status: ");
          let text = if state.is_watching { "Running" } else { "Not running" };
          let color = if state.is_watching {
            egui::Color32::from_rgb(10, 225, 70)
          } else {
            ui.visuals().text_color()
          };
          ui.label(egui::RichText::new(text).color(color))
        });

        ui.horizontal(|ui| {
          ui.label("Villager queued?:");
          let text = if !state.is_watching {
            "Who cares"
          } else if state.is_queued {
            "Yes"
          } else {
            "No"
          };
          let color = if !state.is_watching {
            ui.visuals().text_color()
          } else if state.is_queued {
            egui::Color32::from_rgb(10, 225, 70)
          } else {
            egui::Color32::from_rgb(225, 10, 50)
          };
          ui.label(egui::RichText::new(text).color(color))
        });

        ui.horizontal(|ui| {
          if ui.button(if state.is_watching { "Stop" } else { "Start" }).clicked() {
            state.is_watching = !state.is_watching;
          }

          if ui.button("Reset").clicked() {
            state.config.data = None;
            state.config.y_max = 0;
            state.villager_texture = None;
            state.interface = Interface::WindowSelect;
          }
        });

        ui.add(
          Image::from_texture((
            villager_texture.id(),
            villager_texture.size_vec2()
          ))
            .max_height(100.0)
        );

        egui::ScrollArea::both().show(ui, |ui| {
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

fn image_to_egui(image: &image::RgbaImage) -> egui::ColorImage {
  ColorImage::from_rgba_unmultiplied(
    [image.width() as usize, image.height() as usize],
    image.as_raw()
  )
}

