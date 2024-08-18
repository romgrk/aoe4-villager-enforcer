use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use directories::ProjectDirs;

const DEFAULT_WINDOW_TITLE: &str = "Age of Empires IV ";

#[derive(Debug)]
pub struct Config {
  pub window_title: String,
  pub data: Option<image::GrayImage>,
  pub y_max: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfigOnDisk {
  window_title: String,
  data: Option<(u32, u32, Vec<u8>)>,
  y_max: u32,
}

impl Default for Config {
  fn default() -> Config {
    Config {
      window_title: DEFAULT_WINDOW_TITLE.to_owned(),
      data: None,
      y_max: 0,
    }
  }
}

pub fn load() -> Config {
  let directories = ProjectDirs::from("com", "romgrk", "aoe4-vill-enforcer").unwrap();
  let path = Path::new(directories.config_dir()).join("config.json");

  println!("Path: {:?}", path);

  let content = fs::read_to_string(path);
  if content.is_err() {
    return Default::default();
  }
  let content = content.unwrap();

  let config = serde_json::from_str::<ConfigOnDisk>(&content);
  if config.is_err() {
    return Default::default();
  }
  let config = config.unwrap();

  println!("{:?}", config);

  return Config {
    window_title: config.window_title,
    data: config.data.map(|(w, h, data)| image::GrayImage::from_raw(w, h, data).unwrap()),
    y_max: config.y_max,
  }
}

pub fn write(config: &Config) -> std::io::Result<()> {
  let directories = ProjectDirs::from("com", "romgrk", "aoe4-vill-enforcer").unwrap();
  let path = Path::new(directories.config_dir()).join("config.json");

  fs::create_dir_all(path.parent().unwrap())?;

  let config_on_disk = ConfigOnDisk {
    window_title: config.window_title.to_owned(),
    data: config.data.clone().map(|image| (image.width(), image.height(), image.as_raw().to_owned())),
    y_max: config.y_max,
  };

  return fs::write(
    path,
    serde_json::to_string(&config_on_disk).unwrap(),
  );
}
