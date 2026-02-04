pub mod commands;
pub mod events;

use std::sync::Arc;

use crate::core::world::WorldService;
use crate::generator::ScaffoldService;
use crate::sandbox::runner::SandboxRunner;
use crate::storage::Storage;

pub struct AppState {
  pub storage: Arc<Storage>,
  pub generator: Arc<ScaffoldService>,
  pub sandbox: Arc<SandboxRunner>,
  pub world: Arc<WorldService>,
}
