// eve-abyssal-tracker-tauri/src-tauri/src/abyssal_run_tracker.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, Manager};
use log::*;

use crate::config_manager::ConfigManager;
use crate::eve_log_processor::EveLogProcessor;
use crate::abyssal_data_manager::AbyssalDataManager;
use crate::abyssal_data_analyzer::AbyssalDataAnalyzer;
use crate::log_monitor::LogMonitor;
use crate::system_change_processor::SystemChangeProcessor;

pub struct AbyssalRunTracker {
    config_manager: Arc<Mutex<ConfigManager>>,
    eve_log_processor: Arc<Mutex<EveLogProcessor>>,
    abyssal_data_manager: Arc<Mutex<AbyssalDataManager>>,
    abyssal_data_analyzer: Arc<Mutex<AbyssalDataAnalyzer>>,
    log_monitor: Arc<Mutex<LogMonitor>>,
    system_change_processor: Arc<Mutex<SystemChangeProcessor>>,
    app_handle: AppHandle,
    // 필요한 경우 채널 등을 추가
    // run_end_sender: mpsc::Sender<()>,
    // run_end_receiver: mpsc::Receiver<()>,
}

impl AbyssalRunTracker {
    pub fn new(
        app_handle: AppHandle,
        config_manager: Arc<Mutex<ConfigManager>>,
        eve_log_processor: Arc<Mutex<EveLogProcessor>>,
        abyssal_data_manager: Arc<Mutex<AbyssalDataManager>>,
        abyssal_data_analyzer: Arc<Mutex<AbyssalDataAnalyzer>>,
        log_monitor: Arc<Mutex<LogMonitor>>,
        system_change_processor: Arc<Mutex<SystemChangeProcessor>>,
    ) -> Self {
        // let (run_end_sender, run_end_receiver) = mpsc::channel(1);
        AbyssalRunTracker {
            config_manager,
            eve_log_processor,
            abyssal_data_manager,
            abyssal_data_analyzer,
            log_monitor,
            system_change_processor,
            app_handle,
            // run_end_sender,
            // run_end_receiver,
        }
    }

    // 어비설 런 종료 이벤트를 처리합니다.
    pub async fn _on_abyssal_run_end(&self) {
        info!("Abyssal run ended.");
        // 여기에 런 종료 시 필요한 로직 구현
        // 예: 데이터 저장, 통계 업데이트, UI 알림 등
    }

    // 애플리케이션 종료 로직을 처리합니다.
    pub async fn _exit_application(&self) {
        info!("Exiting application.");
        // 여기에 애플리케이션 종료 시 필요한 정리 로직 구현
        // 예: 모든 스레드 종료, 리소스 해제 등
    }

    // 어비설 런 모니터링을 시작합니다.
    pub async fn start_monitoring(&self) -> Result<(), String> {
        info!("Starting abyssal run monitoring.");
        // 여기에 모니터링 시작 로직 구현
        // LogMonitor, SystemChangeProcessor 등을 사용하여 이벤트 감지 및 처리
        // 예: log_monitor.start_monitoring().await?;
        // system_change_processor.start_monitoring().await?;

        Ok(())
    }
}

#[tauri::command]
pub async fn start_abyssal_run_monitoring_command(app_handle: AppHandle) -> Result<(), String> {
    let tracker = app_handle.state::<AbyssalRunTracker>();
    tracker.start_monitoring().await
}