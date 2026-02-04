#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;
mod core;
mod generator;
mod sandbox;
mod storage;
mod schemas;

use std::sync::Arc;
use tauri::{Manager, Emitter};

use ipc::AppState;
use core::world::WorldService;
use generator::ScaffoldService;
use sandbox::runner::SandboxRunner;
use storage::Storage;

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      let app_handle = app.handle().clone();
      
      let app_dir = match app_handle.path().app_data_dir() {
        Ok(path) => path,
        Err(e) => {
          eprintln!("CRITICAL: Could not resolve app data directory: {}", e);
          return Err(Box::new(e));
        }
      };
      
      let root = app_dir.join("CircuitForge");
      let db_path = root.join("circuitforge.sqlite");
      let artifact_root = root.join("artifacts");
      let runs_root = root.join("runs");

      let db_path_str = db_path.to_string_lossy().into_owned();
      let artifact_root_str = artifact_root.to_string_lossy().into_owned();

      // Use Tauri's runtime to spawn async task
      tauri::async_runtime::spawn(async move {
        // Real error handling for directories
        if let Err(e) = tokio::fs::create_dir_all(&root).await {
          eprintln!("ERROR: Failed to create root directory: {}", e);
          let _ = app_handle.emit("cf.init_error", format!("Filesystem error: {}", e));
          return;
        }
        
        if let Err(e) = tokio::fs::create_dir_all(&runs_root).await {
          eprintln!("ERROR: Failed to create runs directory: {}", e);
          let _ = app_handle.emit("cf.init_error", format!("Filesystem error: {}", e));
          return;
        }
        
        if let Err(e) = tokio::fs::create_dir_all(&artifact_root).await {
          eprintln!("ERROR: Failed to create artifacts directory: {}", e);
          let _ = app_handle.emit("cf.init_error", format!("Filesystem error: {}", e));
          return;
        }

        // Real error handling for storage initialization
        let storage = match Storage::new(&db_path_str, &artifact_root_str).await {
          Ok(s) => Arc::new(s),
          Err(e) => {
            eprintln!("FATAL: Database initialization failed: {}", e);
            let _ = app_handle.emit("cf.init_error", format!("Database initialization failed: {}", e));
            return;
          }
        };

        let sandbox = Arc::new(SandboxRunner::new());
        let generator = Arc::new(ScaffoldService::new());
        let world = Arc::new(WorldService::new(&runs_root));

        let state = AppState { storage, generator, sandbox, world };
        app_handle.manage(state);
        
        println!("CircuitForge Core Services ready.");
        let _ = app_handle.emit("cf.ready", ());
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      ipc::commands::run_create,
      ipc::commands::run_load,
      ipc::commands::run_reset_to_checkpoint,

      ipc::commands::gen_compile_problem,
      ipc::commands::gen_generate_environment,

      ipc::commands::world_observe,
      ipc::commands::world_act,

      ipc::commands::ac_edit_files,
      ipc::commands::ac_create_patch,

      ipc::commands::dc_apply_patch,
      ipc::commands::dc_run_probe,
      ipc::commands::dc_submit_checklist,
      ipc::commands::dc_close_relay,
      ipc::commands::dc_edit_config,

      ipc::commands::construct_define,
      ipc::commands::construct_update_spec,
      ipc::commands::construct_get,
      ipc::commands::construct_evaluate,
      ipc::commands::construct_list,

      ipc::commands::hitl_signoff_write,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}