use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct InitialSettings {
    data_folder: PathBuf,
    global_shortcut: String,
}

#[tauri::command]
pub fn save_initial_settings(settings: InitialSettings) -> Result<(), String> {
    let data_folder = PathBuf::from(&settings.data_folder);
    println!("Data folder: {:?}", data_folder);
    if !data_folder.exists() {
        return Err("Data folder does not exist".to_string());
    }
    
    if !data_folder.is_dir() {
        return Err("Selected path is not a folder".to_string());
    }
    
    // Here you would typically save these settings to a file or database
    // For this example, we'll just print them
    println!("Data folder: {:?}", settings.data_folder);
    println!("Global shortcut: {}", settings.global_shortcut);
    Ok(())
}