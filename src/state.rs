use std::time::Instant;
use egui::TextureHandle;

use crate::config::Config;
use crate::capture::Capture;

#[derive(Copy, Clone)]
pub enum Interface {
  WindowSelect,
  RegionSelect,
  Main,
}

pub struct State {
  pub interface: Interface,
  pub captures: Option<Vec<Capture>>,
  pub window_capture: Option<Capture>,
  pub last_capture: Instant,
  pub config: Config,
  pub villager_texture: Option<TextureHandle>,
  pub is_watching: bool,
  pub is_queued: bool,
}
