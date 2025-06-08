use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use anyhow::{Result, anyhow};
use tauri::{AppHandle, Manager, State};
use configparser::ini::Ini;
use std::sync::Arc; // Arc 추가
use log::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub tracker: TrackerConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub log_path: String,
    pub character_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackerConfig {
    pub abyssal_data_path: String,
    pub daily_stats_path: String,
    pub overall_stats_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub last_abyssal_type: String,
    pub last_ship_class: i32,
}

impl Default for AppConfig {
    fn default() -> Self {
        let username = whoami::username();
        let default_logs_path = format!("C:/Users/{}/Documents/EVE/logs/Chatlogs", username);
        let logs_path = if std::path::Path::new(&default_logs_path).exists() {
            default_logs_path
        } else {
            String::new()
        };

        AppConfig {
            general: GeneralConfig {
                log_path: logs_path,
                character_name: String::new(),
            },
            tracker: TrackerConfig {
                abyssal_data_path: String::from("data"),
                daily_stats_path: String::from(""),
                overall_stats_path: String::from(""),
            },
            ui: UiConfig {
                last_abyssal_type: String::from("T5 Exotic"), // 기본값
                last_ship_class: 1, // 기본값: Cruiser
            },
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    pub config: AppConfig,
}

impl ConfigManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_data_dir = app_handle.path().app_data_dir()
            .or_else(|_| Err(anyhow!("Failed to get app data directory")))?;
        
        // 앱 데이터 디렉토리가 존재하지 않으면 생성
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir)
                .map_err(|e| anyhow!("Failed to create app data directory: {}", e))?;
        }
        
        let config_path = app_data_dir.join("config.ini");

        let mut manager = ConfigManager {
            config_path,
            config: AppConfig::default(),
        };

        if manager.config_path.exists() {
            manager.load()?;
        } else {
            manager.create_default()?;
            manager.save()?;
        }

        Ok(manager)
    }

    pub fn load(&mut self) -> Result<()> {
        let config_str = fs::read_to_string(&self.config_path)?;
        let mut config_ini = Ini::new();
        config_ini.read(config_str).map_err(|e| anyhow!("Failed to read config string: {}", e))?;

        let mut app_config = AppConfig::default();

        if let Some(log_path) = config_ini.get("default", "logs_path") {
            app_config.general.log_path = log_path;
        }
        if let Some(character_name) = config_ini.get("default", "character_name") {
            app_config.general.character_name = character_name;
        }


        if let Some(abyssal_data_path) = config_ini.get("tracker", "abyssal_data_path") {
            app_config.tracker.abyssal_data_path = abyssal_data_path;
        }
        if let Some(daily_stats_path) = config_ini.get("tracker", "daily_stats_path") {
            app_config.tracker.daily_stats_path = daily_stats_path;
        }
        if let Some(overall_stats_path) = config_ini.get("tracker", "overall_stats_path") {
            app_config.tracker.overall_stats_path = overall_stats_path;
        }

        // UI 설정 로드
        if let Some(last_abyssal_type) = config_ini.get("ui", "last_abyssal_type") {
            app_config.ui.last_abyssal_type = last_abyssal_type;
        }
        if let Some(last_ship_class_str) = config_ini.get("ui", "last_ship_class") {
            if let Ok(last_ship_class) = last_ship_class_str.parse::<i32>() {
                app_config.ui.last_ship_class = last_ship_class;
            }
        }

        self.config = app_config;
        self.validate()?;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let mut config_ini = Ini::new();

        config_ini.set("default", "logs_path", Some(self.config.general.log_path.clone()));
        config_ini.set("default", "character_name", Some(self.config.general.character_name.clone()));

        config_ini.set("tracker", "abyssal_data_path", Some(self.config.tracker.abyssal_data_path.clone()));
        config_ini.set("tracker", "daily_stats_path", Some(self.config.tracker.daily_stats_path.clone()));
        config_ini.set("tracker", "overall_stats_path", Some(self.config.tracker.overall_stats_path.clone()));

        // UI 설정 저장
        config_ini.set("ui", "last_abyssal_type", Some(self.config.ui.last_abyssal_type.clone()));
        config_ini.set("ui", "last_ship_class", Some(self.config.ui.last_ship_class.to_string()));

        config_ini.write(&self.config_path).map_err(|e| anyhow!("Failed to write config to file: {}", e))?;
        Ok(())
    }

    pub fn create_default(&mut self) -> Result<()> {
        self.config = AppConfig::default();
                    info!("{} 파일이 생성되었습니다.\nlogs_path와 character_name 값을 확인/수정한 후 프로그램을 다시 실행하세요.", self.config_path.display());
        // Tauri 환경에서는 즉시 종료하지 않고 기본값으로 계속 진행
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        let logs_path = self.config.general.log_path.trim();
        let character_name = self.config.general.character_name.trim();
        
        // Tauri 환경에서는 경고만 출력하고 계속 진행
        if logs_path.is_empty() || character_name.is_empty() {
            warn!("{}에 logs_path와 character_name 값을 입력하면 더 나은 기능을 사용할 수 있습니다.", self.config_path.display());
        }
        Ok(())
    }

    pub fn get_logs_path(&self) -> String {
        self.config.general.log_path.trim().to_string()
    }

    pub fn get_character_name(&self) -> String {
        self.config.general.character_name.trim().to_string()
    }
}

#[tauri::command]
pub async fn get_config(state: State<'_, Arc<tokio::sync::Mutex<ConfigManager>>>) -> Result<AppConfig, String> {
    Ok(state.inner().lock().await.config.clone())
}

#[tauri::command]
pub async fn set_log_path(
    app_handle: AppHandle,
    state: State<'_, Arc<tokio::sync::Mutex<ConfigManager>>>, 
    path: String
) -> Result<(), String> {
    let mut config_manager = state.inner().lock().await;
    config_manager.config.general.log_path = path;
    config_manager.save().map_err(|e| e.to_string())?;
    drop(config_manager);
    
    // LogMonitor 재시작
    crate::restart_log_monitor_if_running(&app_handle).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_character_name(
    app_handle: AppHandle,
    state: State<'_, Arc<tokio::sync::Mutex<ConfigManager>>>, 
    character_name: String
) -> Result<(), String> {
    let mut config_manager = state.inner().lock().await;
    config_manager.config.general.character_name = character_name;
    config_manager.save().map_err(|e| e.to_string())?;
    drop(config_manager);
    
    // LogMonitor 재시작
    crate::restart_log_monitor_if_running(&app_handle).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_ui_config(state: State<'_, Arc<tokio::sync::Mutex<ConfigManager>>>) -> Result<UiConfig, String> {
    Ok(state.inner().lock().await.config.ui.clone())
}

#[tauri::command]
pub async fn set_ui_preferences(
    state: State<'_, Arc<tokio::sync::Mutex<ConfigManager>>>, 
    abyssal_type: String,
    ship_class: i32
) -> Result<(), String> {
    let mut config_manager = state.inner().lock().await;
    config_manager.config.ui.last_abyssal_type = abyssal_type;
    config_manager.config.ui.last_ship_class = ship_class;
    config_manager.save().map_err(|e| e.to_string())?;
    Ok(())
}

