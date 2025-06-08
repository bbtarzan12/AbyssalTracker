use tauri::{AppHandle, Manager, Emitter}; // Manager, Emitter 트레이트 추가
use std::sync::Arc;
use tokio::sync::Mutex;
use polars::prelude::*;
use polars::io::json::JsonWriter;
use chrono::{DateTime, Local};
use serde_json;
use tokio::fs;
use tauri_plugin_log::{Target, TargetKind};
use log::*;

mod config_manager;
use config_manager::ConfigManager;
mod abyssal_data_manager;
use abyssal_data_manager::{AbyssalDataManager, AbyssalResult};
mod eve_api; // eve_api 모듈 선언
use eve_api::EVEApi; // EVEApi 구조체 가져오기
mod eve_log_processor; // eve_log_processor 모듈 선언
use eve_log_processor::EveLogProcessor; // EveLogProcessor 구조체 가져오기
mod log_monitor; // log_monitor 모듈 선언
use log_monitor::LogMonitor; // LogMonitor 구조체 가져오기
mod system_change_processor; // system_change_processor 모듈 선언
use system_change_processor::SystemChangeProcessor;

mod abyssal_data_analyzer;
use abyssal_data_analyzer::{AbyssalDataAnalyzer, AnalysisResult};

mod abyssal_run_tracker;
use abyssal_run_tracker::AbyssalRunTracker;

mod icon_cache; // 아이콘 캐싱 모듈 추가
use icon_cache::IconCache;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[tauri::command]
async fn analyze_abyssal_data_command(app_handle: AppHandle) -> Result<AnalysisResult, String> {
    let abyssal_data_analyzer = app_handle.state::<Arc<Mutex<AbyssalDataAnalyzer>>>();
    
    // AbyssalDataAnalyzer에 AppHandle을 설정
    {
        let mut analyzer = abyssal_data_analyzer.lock().await;
        if analyzer.app_handle.is_none() {
            let eve_api = app_handle.state::<Arc<Mutex<EVEApi>>>();
            let abyssal_data_manager = app_handle.state::<Arc<Mutex<AbyssalDataManager>>>();
            *analyzer = AbyssalDataAnalyzer::new(eve_api.inner().clone(), abyssal_data_manager.inner().clone())
                .with_app_handle(app_handle.clone());
        }
    }
    
    let result = abyssal_data_analyzer.lock().await.analyze_data().await.map_err(|e| e.to_string());
    result
}

#[tauri::command]
async fn start_log_monitor_command(app_handle: AppHandle) -> Result<(), String> {
    let log_monitor = app_handle.state::<Arc<Mutex<LogMonitor>>>();
    let system_change_processor = app_handle.state::<Arc<Mutex<SystemChangeProcessor>>>();
    let scp_arc = Arc::clone(&*system_change_processor);
    let result = log_monitor.lock().await.start(scp_arc).await.map_err(|e| e.to_string());
    
    // 로그 모니터링 상태 이벤트 발송
    let _ = app_handle.emit("log_monitor_status", serde_json::json!({ "status": "started" }));
    
    result
}

#[tauri::command]
async fn stop_log_monitor_command(app_handle: AppHandle) -> Result<(), String> {
    let log_monitor = app_handle.state::<Arc<Mutex<LogMonitor>>>();
    log_monitor.lock().await.stop().await;
    
    // 로그 모니터링 상태 이벤트 발송
    let _ = app_handle.emit("log_monitor_status", serde_json::json!({ "status": "stopped" }));
    
    Ok(())
}

// LogMonitor를 재시작하는 헬퍼 함수
pub async fn restart_log_monitor_if_running(app_handle: &AppHandle) -> Result<(), String> {
    let log_monitor = app_handle.state::<Arc<Mutex<LogMonitor>>>();
    let system_change_processor = app_handle.state::<Arc<Mutex<SystemChangeProcessor>>>();
    
    // 현재 모니터링 중인지 확인
    let is_monitoring = {
        let monitor = log_monitor.lock().await;
        monitor.monitoring
    };
    
    if is_monitoring {
        info!("Configuration changed, restarting LogMonitor...");
        
        // 기존 모니터링 중지
        log_monitor.lock().await.stop().await;
        
        // LogMonitor 재초기화
        {
            let mut monitor = log_monitor.lock().await;
            if let Err(e) = monitor.initialize().await {
                return Err(format!("Failed to reinitialize LogMonitor: {}", e));
            }
        }
        
        // 모니터링 재시작
        let scp_arc = Arc::clone(&*system_change_processor);
        if let Err(e) = log_monitor.lock().await.start(scp_arc).await {
            return Err(format!("Failed to restart LogMonitor: {}", e));
        }
        
        // 상태 이벤트 발송
        let _ = app_handle.emit("log_monitor_status", serde_json::json!({ "status": "restarted" }));
        info!("LogMonitor restarted successfully");
    }
    
    Ok(())
}

#[tauri::command]
async fn find_all_log_files_command(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let eve_log_processor = app_handle.state::<Arc<Mutex<EveLogProcessor>>>();
    let files = eve_log_processor.lock().await.find_all_log_files();
    Ok(files.into_iter().map(|p| p.to_string_lossy().into_owned()).collect())
}

#[tauri::command]
async fn detect_character_name_command(app_handle: AppHandle, file_path: String) -> Result<Option<String>, String> {
    let eve_log_processor = app_handle.state::<Arc<Mutex<EveLogProcessor>>>();
    let path = std::path::PathBuf::from(file_path);
    let result = eve_log_processor.lock().await.detect_character_name(Some(&path));
    Ok(result)
}

#[tauri::command]
async fn load_abyssal_results_command(app_handle: AppHandle) -> Result<Vec<AbyssalResult>, String> {
    let abyssal_data_manager = app_handle.state::<Arc<Mutex<AbyssalDataManager>>>();
    let mut df = abyssal_data_manager.lock().await.load_abyssal_results().map_err(|e| e.to_string())?;
    let mut ndjson_buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut ndjson_buffer);
    JsonWriter::new(&mut cursor).finish(&mut df).map_err(|e| e.to_string())?;
    let ndjson = String::from_utf8(ndjson_buffer).map_err(|e| e.to_string())?;
    let results: Vec<AbyssalResult> = ndjson.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();
    Ok(results)
}

#[tauri::command]
async fn save_abyssal_result_command(
    app_handle: AppHandle, 
    start_time: String, 
    end_time: String, 
    acquired_items: String, 
    abyssal_type: String,
    ship_class: i32
) -> Result<(), String> {
    let abyssal_data_manager = app_handle.state::<Arc<Mutex<AbyssalDataManager>>>();
    
    // 문자열을 DateTime으로 변환
    let start_dt = chrono::DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S %z")
        .map_err(|e| format!("Failed to parse start_time: {}", e))?
        .with_timezone(&chrono::Local);
    let end_dt = chrono::DateTime::parse_from_str(&end_time, "%Y-%m-%d %H:%M:%S %z")
        .map_err(|e| format!("Failed to parse end_time: {}", e))?
        .with_timezone(&chrono::Local);
    
    let result = abyssal_data_manager.lock().await.save_abyssal_result(start_dt, end_dt, acquired_items, abyssal_type, ship_class)
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
async fn save_abyssal_result(
    app_handle: AppHandle,
    abyssal_type: String,
    items: String,
    start_time: String,
    end_time: String,
    _duration: String,
    ship_class: i32
) -> Result<(), String> {
    let abyssal_data_manager = app_handle.state::<Arc<Mutex<AbyssalDataManager>>>();
    
    // 시간 문자열을 NaiveTime으로 변환 (KST)
    let start_time_naive = chrono::NaiveTime::parse_from_str(&start_time, "%H:%M:%S")
        .map_err(|e| format!("Failed to parse start_time '{}': {}", start_time, e))?;
    let end_time_naive = chrono::NaiveTime::parse_from_str(&end_time, "%H:%M:%S")
        .map_err(|e| format!("Failed to parse end_time '{}': {}", end_time, e))?;
    
    // 오늘 날짜로 DateTime 생성
    let today = chrono::Local::now().date_naive();
    let start_datetime = today.and_time(start_time_naive).and_local_timezone(chrono::Local).unwrap();
    let end_datetime = today.and_time(end_time_naive).and_local_timezone(chrono::Local).unwrap();
    
    let result = abyssal_data_manager.lock().await
        .save_abyssal_result(start_datetime, end_datetime, items, abyssal_type, ship_class)
        .map_err(|e| e.to_string());
    
    match &result {
        Ok(_) => {
            info!("Abyssal result saved successfully");
            // 새 런이 저장되면 프론트엔드에 이벤트 발생
            let _ = app_handle.emit("abyssal_run_completed", ());
                         info!("Emitted abyssal_run_completed event");
        },
                 Err(e) => warn!("Failed to save abyssal result: {}", e),
    }
    
    result
}

#[tauri::command]
async fn open_abyssal_result_window(
    app_handle: AppHandle,
    start_time: String,
    end_time: String,
    duration: String,
) -> Result<(), String> {
    // URL 파라미터 생성
    let url = format!(
        "abyssal-result.html?start_time={}&end_time={}&duration={}",
        urlencoding::encode(&start_time),
        urlencoding::encode(&end_time),
        urlencoding::encode(&duration)
    );
    
    // 새 윈도우 생성 (Tauri 2 방식)
    let webview_url = tauri::WebviewUrl::App(url.into());
    let window = tauri::WebviewWindowBuilder::new(&app_handle, "abyssal-result", webview_url)
        .title("🚀 어비셜 런 완료!")
        .inner_size(520.0, 600.0)
        .min_inner_size(480.0, 600.0)
        .resizable(false)
        .center()
        .always_on_top(true)
        .decorations(false)
        .build()
        .map_err(|e| format!("Failed to create abyssal result window: {}", e))?;
    
    // 윈도우 포커스
    let _ = window.set_focus();
    
    info!("Abyssal result window opened successfully");
    Ok(())
}

#[tauri::command]
async fn test_abyssal_window(app_handle: AppHandle) -> Result<(), String> {
    use chrono::{Local, Timelike};
    use rand::Rng;
    
    // 랜덤 값들을 미리 생성 (Send 문제 해결)
    let (start_hour, start_minute, start_second, duration_minutes, duration_seconds) = {
        let mut rng = rand::thread_rng();
        (
            rng.gen_range(10..23), // 10시부터 22시까지
            rng.gen_range(0..60),
            rng.gen_range(0..60),
            rng.gen_range(10..21), // 10분 ~ 20분 사이
            rng.gen_range(0..60)
        )
    };
    
    // 시작 시간 생성
    let today = Local::now().date_naive();
    let start_time = today
        .and_hms_opt(start_hour, start_minute, start_second)
        .unwrap();
    
    // 종료 시간 계산
    let end_time = start_time + chrono::Duration::minutes(duration_minutes) + chrono::Duration::seconds(duration_seconds);
    
    // 문자열 포맷
    let start_time_str = format!("{:02}:{:02}:{:02}", start_time.hour(), start_time.minute(), start_time.second());
    let end_time_str = format!("{:02}:{:02}:{:02}", end_time.hour(), end_time.minute(), end_time.second());
    let duration_str = format!("{}m {}s", duration_minutes, duration_seconds);
    
    if let Err(e) = open_abyssal_result_window(
        app_handle.clone(),
        start_time_str,
        end_time_str,
        duration_str
    ).await {
        warn!("Failed to open test window: {}", e);
        return Err(e);
    }
    
    Ok(())
}

#[tauri::command]
async fn delete_abyssal_run_command(
    app_handle: AppHandle,
    start_time_kst: String,
    end_time_kst: String
) -> Result<(), String> {
    let abyssal_data_manager = app_handle.state::<Arc<Mutex<AbyssalDataManager>>>();
    let manager = abyssal_data_manager.lock().await;
    
    let result = manager.delete_abyssal_run(&start_time_kst, &end_time_kst)
        .map_err(|e| e.to_string());
    
    match &result {
        Ok(_) => info!("Abyssal run deleted successfully"),
        Err(e) => warn!("Failed to delete abyssal run: {}", e),
    }
    
    result
}

#[tauri::command]
async fn light_refresh_abyssal_data_command(app_handle: AppHandle) -> Result<AnalysisResult, String> {
    let abyssal_data_manager = app_handle.state::<Arc<Mutex<AbyssalDataManager>>>();
    let abyssal_data_analyzer = app_handle.state::<Arc<Mutex<AbyssalDataAnalyzer>>>();
    
    // CSV 데이터만 다시 로드
    let df = {
        let manager = abyssal_data_manager.lock().await;
        manager.load_abyssal_results().map_err(|e| e.to_string())?
    };
    
    // 기존 캐시된 가격 정보로 빠른 분석 (새로운 아이템 발견 시에만 API 호출)
    let mut analyzer = abyssal_data_analyzer.lock().await;
    let result = analyzer.light_analyze_data(df).await.map_err(|e| e.to_string())?;
    
    Ok(result)
}

#[tauri::command]
async fn process_log_line_command(app_handle: AppHandle, line: String) -> Result<(), String> {
    let system_change_processor = app_handle.state::<Arc<Mutex<SystemChangeProcessor>>>();
    system_change_processor.lock().await.process_log_line(&line).await;
    Ok(())
}

#[tauri::command]
async fn scan_past_runs_command(app_handle: AppHandle, logs_path: String, character_name: String) -> Result<(), String> {
    let system_change_processor = app_handle.state::<Arc<Mutex<SystemChangeProcessor>>>();
    system_change_processor.lock().await.scan_past_runs(&logs_path, &character_name).await;
    Ok(())
}

// 아이콘 관련 커맨드들
#[tauri::command]
async fn get_type_id(app_handle: AppHandle, item_name: String) -> Result<Option<u32>, String> {
    let icon_cache = app_handle.state::<Arc<Mutex<IconCache>>>();
    let cache = icon_cache.lock().await;
    Ok(cache.get_type_id(&item_name))
}

#[tauri::command]
async fn get_filament_name(app_handle: AppHandle, abyssal_type: String) -> Result<Option<String>, String> {
    let abyssal_data_manager = app_handle.state::<Arc<Mutex<AbyssalDataManager>>>();
    let manager = abyssal_data_manager.lock().await;
    Ok(manager.abyssal_type_to_filament_name(&abyssal_type))
}

#[tauri::command]
async fn get_icon_url(type_id: u32) -> Result<String, String> {
    Ok(format!("https://images.evetech.net/types/{}/icon", type_id))
}

#[tauri::command]
async fn get_best_image_url(app_handle: AppHandle, type_id: u32, item_name: String) -> Result<String, String> {
    let icon_cache = app_handle.state::<Arc<Mutex<IconCache>>>();
    let cache = icon_cache.lock().await;
    cache.get_best_image_url(type_id, &item_name).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_for_update_command(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let current_version = app_handle.package_info().version.to_string();
    
    // GitHub API를 통해 최신 버전 정보 가져오기
    let latest_version = match get_latest_github_version().await {
        Ok(version) => version,
        Err(e) => {
            warn!("Failed to get latest version from GitHub: {}", e);
            return Err(format!("업데이트 확인 실패: {}", e));
        }
    };
    
    info!("Version comparison - Current: '{}', Latest: '{}'", current_version, latest_version);
    
    // 버전 비교 (semantic version)
    let needs_update = compare_versions(&current_version, &latest_version);
    
    info!("Version comparison result: needs_update = {}", needs_update);
    
    if needs_update {
        info!("Update available - Current: {}, Latest: {}", current_version, latest_version);
    } else {
        info!("No updates available - Current: {}, Latest: {} (already up to date)", current_version, latest_version);
    }
    
    Ok(serde_json::json!({
        "available": needs_update,
        "latest_version": latest_version
    }))
}

// 버전 비교 함수 (semantic version)
fn compare_versions(current: &str, latest: &str) -> bool {
    fn parse_version(version: &str) -> Vec<u32> {
        version.split('.')
            .map(|s| s.parse::<u32>().unwrap_or(0))
            .collect()
    }
    
    let current_parts = parse_version(current);
    let latest_parts = parse_version(latest);
    
    info!("Parsed versions - Current: {:?}, Latest: {:?}", current_parts, latest_parts);
    
    for i in 0..std::cmp::max(current_parts.len(), latest_parts.len()) {
        let current_part = current_parts.get(i).unwrap_or(&0);
        let latest_part = latest_parts.get(i).unwrap_or(&0);
        
        if latest_part > current_part {
            info!("Update needed: latest part {} > current part {} at position {}", latest_part, current_part, i);
            return true;
        } else if latest_part < current_part {
            info!("No update needed: latest part {} < current part {} at position {}", latest_part, current_part, i);
            return false;
        }
    }
    
    info!("Versions are equal");
    false
}

async fn get_latest_github_version() -> Result<String, String> {
    let (version, _) = get_latest_github_release().await?;
    Ok(version)
}

async fn get_latest_github_release() -> Result<(String, String), String> {
    use std::collections::HashMap;
    
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/bbtarzan12/AbyssalTracker/releases/latest")
        .header("User-Agent", "EVE-Abyssal-Tracker")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch latest release: {}", e))?;
        
    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }
    
    let json: HashMap<String, serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
    let tag_name = json.get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or("No tag_name found in response")?;
        
    // "v1.0.4" 형태에서 "v" 제거
    let version = if tag_name.starts_with('v') {
        tag_name[1..].to_string()
    } else {
        tag_name.to_string()
    };
    
    // assets에서 .exe 파일 찾기
    let assets = json.get("assets")
        .and_then(|v| v.as_array())
        .ok_or("No assets found in response")?;
    
    let setup_asset = assets.iter()
        .find(|asset| {
            asset.get("name")
                .and_then(|name| name.as_str())
                .map(|name| name.ends_with("-setup.exe"))
                .unwrap_or(false)
        })
        .ok_or("No setup.exe file found in release assets")?;
    
    let download_url = setup_asset.get("browser_download_url")
        .and_then(|url| url.as_str())
        .ok_or("No download URL found for setup.exe")?;
    
    Ok((version, download_url.to_string()))
}

#[tauri::command]
async fn download_and_install_update_command(app_handle: AppHandle) -> Result<String, String> {
    let current_version = app_handle.package_info().version.to_string();
    
    // GitHub API를 통해 최신 버전 정보와 다운로드 URL 가져오기
    let (latest_version, download_url) = match get_latest_github_release().await {
        Ok((version, url)) => (version, url),
        Err(e) => {
            warn!("Failed to get latest version from GitHub during install: {}", e);
            return Err(format!("GitHub API 오류: {}", e));
        }
    };
    
    // 수동 버전 비교로 업데이트 필요성 확인
    let needs_update = compare_versions(&current_version, &latest_version);
    
    if !needs_update {
        info!("No update needed - Current: {}, Latest: {}", current_version, latest_version);
        return Ok("이미 최신 버전입니다".to_string());
    }
    
    info!("Downloading and installing update - Current: {}, Latest: {}", current_version, latest_version);
    
    // 임시 폴더에 설치 파일 다운로드
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(format!("EVE_Abyssal_Tracker_{}_x64-setup.exe", latest_version));
    
    // 다운로드
    let client = reqwest::Client::new();
    let response = client.get(&download_url).send().await
        .map_err(|e| format!("다운로드 실패: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("다운로드 오류: HTTP {}", response.status()));
    }
    
    let bytes = response.bytes().await
        .map_err(|e| format!("다운로드 데이터 읽기 실패: {}", e))?;
    
    // 파일 저장
    std::fs::write(&installer_path, bytes)
        .map_err(|e| format!("설치 파일 저장 실패: {}", e))?;
    
    info!("Downloaded installer to: {}", installer_path.display());
    
    // 설치 파일 실행 (일반적인 더블클릭 방식)
    info!("Launching installer: {}", installer_path.display());
    
    match std::process::Command::new(&installer_path).spawn() {
        Ok(_) => {
            info!("Installer launched successfully, exiting application immediately");
            
            // 설치 프로그램이 시작되면 즉시 앱 종료
            std::thread::spawn(move || {
                std::process::exit(0);
            });
            
            Ok("업데이트 설치 프로그램이 시작되었습니다.".to_string())
        },
        Err(e) => {
            warn!("Failed to launch installer: {}", e);
            // 파일 정리
            let _ = std::fs::remove_file(&installer_path);
            Err(format!("설치 파일 실행 실패: {}", e))
        }
    }
}

#[tauri::command]
async fn get_csv_data_path(app_handle: AppHandle) -> Result<String, String> {
    let data_dir_path = match app_handle.path().app_data_dir() {
        Ok(app_data_dir) => {
            let data_dir = app_data_dir.join("data");
            // 디렉토리가 없으면 생성
            if let Err(e) = std::fs::create_dir_all(&data_dir) {
                warn!("Warning: Failed to create app data directory: {}", e);
                // 실패 시 현재 디렉토리의 data 폴더 사용
                std::path::PathBuf::from("data")
            } else {
                data_dir
            }
        },
        Err(e) => {
            warn!("Warning: Failed to get app data directory: {}, using local data directory", e);
            std::path::PathBuf::from("data")
        }
    };
    
    Ok(data_dir_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn export_daily_analysis(
    app_handle: AppHandle,
    selected_date: String,
    format: String, // "csv" or "json"
) -> Result<String, String> {
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
    
    // 분석 데이터 가져오기
    let abyssal_data_analyzer = app_handle.state::<Arc<Mutex<AbyssalDataAnalyzer>>>();
    let analysis_result = abyssal_data_analyzer.lock().await.analyze_data().await
        .map_err(|e| format!("Failed to analyze data: {}", e))?;
    
    // 선택된 날짜의 데이터 필터링
    let daily_data = analysis_result.daily_stats.get(&selected_date)
        .ok_or_else(|| format!("No data found for date: {}", selected_date))?;
    
    // CSV 형식만 지원
    if format != "csv" {
        return Err("Only CSV format is supported".to_string());
    }
    let file_extension = "csv";
    
    // 파일 저장 대화상자 열기
    let file_path = app_handle.dialog()
        .file()
        .set_title("일별 분석 데이터 내보내기")
        .set_file_name(&format!("abyssal_daily_analysis_{}.{}", selected_date, file_extension))
        .add_filter("Data files", &[file_extension])
        .blocking_save_file();
    
    if let Some(path) = file_path {
        // FilePath를 PathBuf로 변환
        let path_buf = std::path::PathBuf::from(path.to_string());
        
        // CSV 형식으로 변환 (순수한 테이블 데이터만)
        let mut csv_content = String::new();
        csv_content.push_str("시작시각(KST),종료시각(KST),런 소요(분),어비셜 종류,함급,실수익,ISK/h,획득 아이템,드롭,입장료\n");
        
        for run in &daily_data.runs {
            csv_content.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                run.start_time,
                run.end_time,
                run.run_time_minutes,
                run.abyssal_type,
                run.ship_class,
                run.net_profit,
                run.isk_per_hour,
                run.acquired_items.replace(",", ";"), // CSV 구분자 충돌 방지
                run.drop_value,
                run.entry_cost
            ));
        }
        
        let content = csv_content;
        
        // 파일 저장
        fs::write(&path_buf, content).await
            .map_err(|e| format!("Failed to write file: {}", e))?;
        
        // 성공 메시지 표시
        app_handle.dialog()
            .message(format!("일별 분석 데이터가 성공적으로 내보내졌습니다:\n{}", path_buf.display()))
            .kind(MessageDialogKind::Info)
            .title("내보내기 완료")
            .blocking_show();
        
        Ok(path_buf.to_string_lossy().to_string())
    } else {
        Err("Export cancelled by user".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    Target::new(TargetKind::LogDir {
                        file_name: Some("eve_abyssal_tracker".to_string()),
                    }),
                    Target::new(TargetKind::Stdout),
                ])
                .build()
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Python과 동일한 초기화 순서
                
                // 1. ConfigManager 초기화 (Python의 ConfigManager와 동일)
                let config_manager = Arc::new(Mutex::new(
                    ConfigManager::new(&app_handle).expect("Failed to initialize ConfigManager")
                ));
                app_handle.manage(config_manager.clone());

                // 2. AbyssalDataManager 초기화
                let abyssal_data_manager = Arc::new(Mutex::new(
                    AbyssalDataManager::new(app_handle.clone())
                ));
                app_handle.manage(abyssal_data_manager.clone());

                // 3. EVEApi 초기화
                let eve_api = Arc::new(Mutex::new(
                    EVEApi::new(&app_handle).await.expect("Failed to initialize EVEApi")
                ));
                app_handle.manage(eve_api.clone());

                // 4. IconCache 초기화
                // 앱 데이터 디렉토리 사용
                let data_dir = match app_handle.path().app_data_dir() {
                    Ok(app_data_dir) => {
                        let data_dir = app_data_dir.join("data");
                        if let Err(e) = std::fs::create_dir_all(&data_dir) {
                            warn!("Warning: Failed to create app data directory for IconCache: {}", e);
                            std::path::PathBuf::from("data")
                        } else {
                            data_dir
                        }
                    },
                    Err(e) => {
                        warn!("Warning: Failed to get app data directory for IconCache: {}, using local data directory", e);
                        std::path::PathBuf::from("data")
                    }
                };
                let mut icon_cache = IconCache::new(data_dir);
                if let Err(e) = icon_cache.initialize().await {
                    error!("Failed to initialize IconCache: {}", e);
                }
                let icon_cache_arc = Arc::new(Mutex::new(icon_cache));
                app_handle.manage(icon_cache_arc);

                // 5. AbyssalDataAnalyzer 초기화
                let abyssal_data_analyzer = Arc::new(Mutex::new(
                    AbyssalDataAnalyzer::new(eve_api.clone(), abyssal_data_manager.clone())
                ));
                app_handle.manage(abyssal_data_analyzer.clone());

                // 6. EveLogProcessor 초기화 (Python과 동일)
                let config_lock = config_manager.lock().await;
                let logs_path = std::path::PathBuf::from(config_lock.get_logs_path());
                drop(config_lock);
                
                let eve_log_processor = Arc::new(Mutex::new(
                    EveLogProcessor::new(logs_path.clone(), None)
                ));
                app_handle.manage(eve_log_processor.clone());

                // 7. SystemChangeProcessor 초기화 (Python과 동일한 콜백 구조)
                let abyssal_data_manager_for_callback = abyssal_data_manager.clone();
                let app_handle_for_callback = app_handle.clone();
                let on_abyssal_run_end = Box::new(move |start_time: DateTime<Local>, end_time: DateTime<Local>| {
                    // Python의 _on_abyssal_run_end와 동일한 로직
                    info!("Abyssal run ended: {} to {}", start_time, end_time);
                    
                    // 런 시간 계산
                    let duration = end_time.signed_duration_since(start_time);
                    let duration_str = format!("{}m {}s", 
                        duration.num_minutes(), 
                        duration.num_seconds() % 60);
                    
                    // 새 윈도우 열기 (원본 Python의 팝업과 동일)
                    let app_handle_clone = app_handle_for_callback.clone();
                    let start_time_str = start_time.format("%H:%M:%S").to_string();
                    let end_time_str = end_time.format("%H:%M:%S").to_string();
                    let duration_str_clone = duration_str.clone();
                    
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = crate::open_abyssal_result_window(
                            app_handle_clone,
                            start_time_str,
                            end_time_str,
                            duration_str_clone
                        ).await {
                            warn!("Failed to open abyssal result window: {}", e);
                        }
                    });
                });

                let system_change_processor = Arc::new(Mutex::new(
                    SystemChangeProcessor::new(eve_log_processor.clone(), Some(on_abyssal_run_end))
                ));
                app_handle.manage(system_change_processor.clone());

                // 8. LogMonitor 초기화 (Python과 동일한 콜백 구조)
                let system_change_processor_for_callback = system_change_processor.clone();
                let config_manager_for_callback = config_manager.clone();
                
                let on_new_log_lines = Box::new(move |lines: Vec<String>| {
                    // Python의 _on_new_log_lines와 동일한 로직
                    let system_change_processor = system_change_processor_for_callback.clone();
                    tauri::async_runtime::spawn(async move {
                        for line in lines {
                            system_change_processor.lock().await.process_log_line(&line).await;
                        }
                    });
                });

                let system_change_processor_for_file_change = system_change_processor.clone();
                let on_log_file_change = Box::new(move || {
                    // Python의 _on_log_file_change와 동일한 로직
                    info!("Log file changed. Re-scanning past runs.");
                    let system_change_processor = system_change_processor_for_file_change.clone();
                    let config_manager = config_manager_for_callback.clone();
                    tauri::async_runtime::spawn(async move {
                        let config_lock = config_manager.lock().await;
                        let logs_path = config_lock.get_logs_path();
                        let character_name = config_lock.get_character_name();
                        drop(config_lock);
                        
                        system_change_processor.lock().await.scan_past_runs(&logs_path, &character_name).await;
                        system_change_processor.lock().await.print_past_runs();
                    });
                });

                let mut log_monitor = LogMonitor::new(
                    config_manager.clone(),
                    Some(on_new_log_lines),
                    Some(on_log_file_change),
                );
                
                // Python과 동일한 초기화
                if let Err(e) = log_monitor.initialize().await {
                    error!("Failed to initialize LogMonitor: {}", e);
                }
                
                let log_monitor_arc = Arc::new(Mutex::new(log_monitor));
                app_handle.manage(log_monitor_arc.clone());
                
                // LogMonitor는 설정에서 수동으로 시작하도록 변경
                info!("LogMonitor initialized, ready to start manually from settings.");

                // 9. AbyssalRunTracker 초기화 (Python의 tracker.py와 동일)
                let abyssal_run_tracker = AbyssalRunTracker::new(
                    app_handle.clone(),
                    config_manager.clone(),
                    eve_log_processor.clone(),
                    abyssal_data_manager.clone(),
                    abyssal_data_analyzer.clone(),
                    log_monitor_arc.clone(),
                    system_change_processor.clone(),
                );
                app_handle.manage(abyssal_run_tracker);

                // Python과 동일하게 초기 스캔 수행 (에러가 발생해도 계속 진행)
                let config_manager_clone = config_manager.clone();
                let system_change_processor_clone = system_change_processor.clone();
                
                tokio::spawn(async move {
                    if let Ok(config_lock) = config_manager_clone.try_lock() {
                        let logs_path = config_lock.get_logs_path();
                        let character_name = config_lock.get_character_name();
                        drop(config_lock);
                        
                        if !logs_path.is_empty() && !character_name.is_empty() {
                            if let Ok(mut processor) = system_change_processor_clone.try_lock() {
                                processor.scan_past_runs(&logs_path, &character_name).await;
                                processor.print_past_runs();
                            }
                        }
                    } else {
                        info!("Could not access config for initial scan, but continuing with application startup.");
                    }
                });
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            analyze_abyssal_data_command,
            light_refresh_abyssal_data_command,
            config_manager::get_config,
            config_manager::set_log_path,
            config_manager::set_character_name,
            config_manager::get_ui_config,
            config_manager::set_ui_preferences,
            load_abyssal_results_command,
            save_abyssal_result_command,
            save_abyssal_result,
            delete_abyssal_run_command,
            open_abyssal_result_window,
            test_abyssal_window,
            eve_api::get_type_ids,
            eve_api::get_fuzzwork_prices,
            find_all_log_files_command,
            detect_character_name_command,
            start_log_monitor_command,
            stop_log_monitor_command,
            process_log_line_command,
            scan_past_runs_command,
            abyssal_run_tracker::start_abyssal_run_monitoring_command,
            get_type_id,
            get_filament_name,
            get_icon_url,
            get_best_image_url,
            get_csv_data_path,
            export_daily_analysis,
            check_for_update_command,
            download_and_install_update_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
