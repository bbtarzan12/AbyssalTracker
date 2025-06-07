use tauri::{AppHandle, Manager, Emitter}; // Manager, Emitter 트레이트 추가
use std::sync::Arc;
use tokio::sync::Mutex;
use polars::prelude::*;
use polars::io::json::JsonWriter;
use chrono::{DateTime, Local};

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
            let eve_api = app_handle.state::<Arc<EVEApi>>();
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
        println!("[INFO] Configuration changed, restarting LogMonitor...");
        
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
        println!("[INFO] LogMonitor restarted successfully");
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
    abyssal_type: String
) -> Result<(), String> {
    let abyssal_data_manager = app_handle.state::<Arc<Mutex<AbyssalDataManager>>>();
    
    // 문자열을 DateTime으로 변환
    let start_dt = chrono::DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S %z")
        .map_err(|e| format!("Failed to parse start_time: {}", e))?
        .with_timezone(&chrono::Local);
    let end_dt = chrono::DateTime::parse_from_str(&end_time, "%Y-%m-%d %H:%M:%S %z")
        .map_err(|e| format!("Failed to parse end_time: {}", e))?
        .with_timezone(&chrono::Local);
    
    let result = abyssal_data_manager.lock().await.save_abyssal_result(start_dt, end_dt, acquired_items, abyssal_type)
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
    duration: String
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
        .save_abyssal_result(start_datetime, end_datetime, items, abyssal_type)
        .map_err(|e| e.to_string());
    
    match &result {
        Ok(_) => {
            println!("[INFO] Abyssal result saved successfully");
            // 새 런이 저장되면 프론트엔드에 이벤트 발생
            let _ = app_handle.emit("abyssal_run_completed", ());
            println!("[INFO] Emitted abyssal_run_completed event");
        },
        Err(e) => println!("[ERROR] Failed to save abyssal result: {}", e),
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
    
    println!("[INFO] Abyssal result window opened successfully");
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
        println!("[ERROR] Failed to open test window: {}", e);
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
        Ok(_) => println!("[INFO] Abyssal run deleted successfully"),
        Err(e) => println!("[ERROR] Failed to delete abyssal run: {}", e),
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
                let eve_api = Arc::new(
                    EVEApi::new(&app_handle).await.expect("Failed to initialize EVEApi")
                );
                app_handle.manage(eve_api.clone());

                // 4. IconCache 초기화
                // 실제 파일 위치는 eve-abyssal-tracker-tauri/src-tauri/data/typeid_cache.json
                let data_dir = std::path::PathBuf::from("data");
                let mut icon_cache = IconCache::new(data_dir);
                if let Err(e) = icon_cache.initialize().await {
                    eprintln!("Failed to initialize IconCache: {}", e);
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
                let language = Some(config_lock.get_language()).filter(|s| !s.is_empty());
                drop(config_lock);
                
                let eve_log_processor = Arc::new(Mutex::new(
                    EveLogProcessor::new(logs_path.clone(), language.clone())
                ));
                app_handle.manage(eve_log_processor.clone());

                // 7. SystemChangeProcessor 초기화 (Python과 동일한 콜백 구조)
                let abyssal_data_manager_for_callback = abyssal_data_manager.clone();
                let app_handle_for_callback = app_handle.clone();
                let on_abyssal_run_end = Box::new(move |start_time: DateTime<Local>, end_time: DateTime<Local>| {
                    // Python의 _on_abyssal_run_end와 동일한 로직
                    println!("Abyssal run ended: {} to {}", start_time, end_time);
                    
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
                            println!("[ERROR] Failed to open abyssal result window: {}", e);
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
                    println!("[INFO] Log file changed. Re-scanning past runs.");
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
                    eprintln!("Failed to initialize LogMonitor: {}", e);
                }
                
                let log_monitor_arc = Arc::new(Mutex::new(log_monitor));
                app_handle.manage(log_monitor_arc.clone());
                
                // LogMonitor는 설정에서 수동으로 시작하도록 변경
                println!("[INFO] LogMonitor initialized, ready to start manually from settings.");

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
                        println!("[WARNING] Could not access config for initial scan, but continuing with application startup.");
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
            config_manager::set_language,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
