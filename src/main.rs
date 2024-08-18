use eframe::egui;
use egui::{Image, ColorImage, Layout, TextureHandle};
use xcap::Window;
use image::DynamicImage;

mod contour;
use contour::detect_squares;

fn main() -> eframe::Result {
  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 700.0]),
    ..Default::default()
  };
  eframe::run_native(
    "AOE4 Villager Enforcer",
    options,
    Box::new(|cc| {
      // This gives us image support:
      egui_extras::install_image_loaders(&cc.egui_ctx);
      Ok(Box::<EnforcerApp>::default())
    }),
  )
}

struct EnforcerApp {
  capture_index: i64,
  captures: Option<Vec<WindowScreenshot>>,
  region_select_image: Option<TextureHandle>,
}

impl Default for EnforcerApp {
    fn default() -> EnforcerApp {
        EnforcerApp {
            capture_index: -1,
            captures: None,
            region_select_image: None,
        }
    }
}

impl eframe::App for EnforcerApp {
  fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    if self.captures.is_none() {
      self.captures = Some(take_screenshot());
    }

    if self.capture_index == -1 {
      self.ui_window_select(ctx, frame);
    } else {
      self.ui_region_select(ctx, frame);
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
                let color_image = ColorImage::from_rgba_unmultiplied(
                  [capture.data.width() as usize, capture.data.height() as usize],
                  capture.data.as_raw()
                );

                ui.ctx().load_texture(
                  capture.title.clone(),
                  color_image,
                  Default::default(),
                )
              });

              if ui.button(&capture.title).clicked() {
                self.capture_index = index
              }
              ui.add(
                Image::from_texture((texture.id(), texture.size_vec2()))
                  .max_height(200.0)
              );
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

    let contour_texture = self.region_select_image.get_or_insert_with(|| {
      let source_image = &capture.data;

      let processing = source_image;
      let processing = image::imageops::colorops::grayscale(processing);
      let processing = {
        let n = 75;
        imageproc::contrast::stretch_contrast(&processing, n, n + 1, 0, 255)
      };

      let processed_image = processing;

      let contours = imageproc::contours::find_contours::<i32>(&processed_image);

      let squares = detect_squares(
        processed_image.width(),
        processed_image.height(),
        &contours
      );

      // let mut image = DynamicImage::ImageLuma8(processed_image).to_rgba8();
      let mut image = source_image.clone();

      for contour in contours.iter() {
        for point in &contour.points {
          image.put_pixel(
            point.x as u32,
            point.y as u32,
            image::Rgba([255, 255, 0, 255])
          );
        }
      }
      for square in squares.iter() {
        for point in &square.contour.points {
          image.put_pixel(
            point.x as u32,
            point.y as u32,
            image::Rgba([255, 0, 0, 255])
          );
        }
      }

      let color_image = ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw()
      );

      ctx.load_texture(
        format!("{}-contoured", capture.title.clone()),
        color_image,
        Default::default(),
      )
    });

    egui::CentralPanel::default().show(ctx, |ui| {
      egui::ScrollArea::both().show(ui, |ui| {

        ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
          ui.vertical(|ui| {
            ui.add(
              Image::from_texture((contour_texture.id(), contour_texture.size_vec2()))
                // .max_height(600.0)
            );
          });
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

fn take_screenshot() -> Vec<WindowScreenshot> {
  let windows = Window::all().unwrap();
  let mut results = vec![];

  for window in windows {
    println!(
      "Window: {:?} {:?}",
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
