use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader, AsyncSeekExt},
    sync::{mpsc, Mutex},
    task::JoinHandle,
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use crossbeam_channel as channel;
use encoding_rs::UTF_16LE;
use log::{info, warn, error};

use crate::config_manager::ConfigManager;
use crate::eve_log_processor::EveLogProcessor;
use crate::system_change_processor::SystemChangeProcessor;

pub struct LogMonitor {
    config_manager: Arc<Mutex<ConfigManager>>,
    log_processor: EveLogProcessor, // 더 이상 Arc<Mutex>가 아님!
    logs_path: PathBuf,
    character_name: String,
    language: Option<String>,
    log_file: Option<PathBuf>,
    last_position: u64,
    last_line_count: usize,  // 마지막으로 읽은 라인 수
    pub monitoring: bool,
    observer: Option<RecommendedWatcher>,
    monitor_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    stop_signal_sender: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    // Python과 동일한 콜백 구조
    on_new_log_lines: Option<Box<dyn Fn(Vec<String>) + Send + Sync>>,
    on_log_file_change: Option<Box<dyn Fn() + Send + Sync>>,
}

impl LogMonitor {
    pub fn new(
        config_manager: Arc<Mutex<ConfigManager>>,
        on_new_log_lines: Option<Box<dyn Fn(Vec<String>) + Send + Sync>>,
        on_log_file_change: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> Self {
        LogMonitor {
            config_manager,
            log_processor: EveLogProcessor::new(PathBuf::new(), None), // 자체 인스턴스 생성
            logs_path: PathBuf::new(),
            character_name: String::new(),
            language: None,
            log_file: None,
            last_position: 0,
            last_line_count: 0,
            monitoring: false,
            observer: None,
            monitor_task: Arc::new(Mutex::new(None)),
            stop_signal_sender: Arc::new(Mutex::new(None)),
            on_new_log_lines,
            on_log_file_change,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Python과 동일한 초기화 로직
        let config_manager = self.config_manager.lock().await;
        self.logs_path = PathBuf::from(config_manager.get_logs_path());
        self.character_name = config_manager.get_character_name();
        self.language = None;
        drop(config_manager);

        // log_processor 설정 - 더 이상 lock 필요 없음!
        self.log_processor.logs_path = self.logs_path.clone();
        if let Some(ref language) = self.language {
            self.log_processor.language = Some(language.clone());
        }

        Ok(())
    }

    async fn find_latest_local_log(&mut self) -> Option<PathBuf> {
        let files = self.log_processor.find_all_log_files();
        
        if files.is_empty() {
            println!("[INFO] No local chat log files found in '{}'.", self.logs_path.display());
            return None;
        }

        // 파일과 수정 시간을 수집
        let mut files_with_mtime: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        for file in files {
            if let Ok(metadata) = std::fs::metadata(&file) {
                if let Ok(mtime) = metadata.modified() {
                    files_with_mtime.push((file, mtime));
                }
            }
        }
        files_with_mtime.sort_by(|a, b| b.1.cmp(&a.1)); // 최신 순으로 정렬

        // 캐릭터 이름이 설정된 경우: 해당 캐릭터의 최신 로그 찾기
        if !self.character_name.is_empty() {
            for (file_path, mtime) in &files_with_mtime {
                // 각 파일에서 캐릭터 이름 확인
                let detected_name = self.log_processor.detect_character_name(Some(file_path));
                if let Some(detected) = detected_name {
                    if detected == self.character_name {
                        println!("[INFO] Found matching log for character '{}': {} (modified: {:?})", 
                            self.character_name,
                            file_path.file_name().unwrap_or_default().to_string_lossy(),
                            mtime
                        );
                        self.log_processor.set_log_file(file_path.clone());
                        return Some(file_path.clone());
                    }
                }
            }
            
            println!("[WARNING] No log files found for character '{}'. Available files:", self.character_name);
            for (file_path, _) in files_with_mtime.iter().take(5) {
                let detected_name = self.log_processor.detect_character_name(Some(file_path));
                println!("  - {} (character: {:?})", 
                    file_path.file_name().unwrap_or_default().to_string_lossy(),
                    detected_name
                );
            }
            return None;
        }
        
        // 캐릭터 이름이 설정되지 않은 경우: 최신 파일에서 자동 감지
        if let Some((latest_file, _)) = files_with_mtime.first() {
            self.log_processor.set_log_file(latest_file.clone());
            
            let detected_name = self.log_processor.detect_character_name(Some(latest_file));
            if let Some(detected) = detected_name {
                self.character_name = detected.clone();
                println!("[INFO] Auto-detected character name: '{}' from {}", 
                    self.character_name, 
                    latest_file.file_name().unwrap_or_default().to_string_lossy()
                );
                Some(latest_file.clone())
            } else {
                println!("[WARNING] Could not auto-detect character name from: '{}'",
                    latest_file.file_name().unwrap_or_default().to_string_lossy()
                );
                None
            }
        } else {
            None
        }
    }

    async fn process_new_lines(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.log_file.is_none() {
            return Ok(());
        }

        if self.log_processor.patterns.is_empty() {
            return Ok(());
        }

        let file_path = self.log_file.as_ref().unwrap();
        
        // Python과 동일한 UTF-16LE 인코딩 처리
        let file_content = tokio::fs::read(file_path).await?;
        let (cow, _, _) = UTF_16LE.decode(&file_content);
        let content = cow.into_owned();
        
        let lines: Vec<&str> = content.lines().collect();

        // 현재 파일의 총 라인 수와 이전에 읽은 라인 수 비교
        if lines.len() < self.last_line_count {
            // 파일이 잘렸거나 새로 시작된 경우
            warn!("로그 파일이 잘렸거나 새로 시작되었습니다. 라인 카운트를 0으로 재설정합니다.");
            self.last_line_count = 0;
        }

        // 새로운 라인이 있는지 확인
        if lines.len() == self.last_line_count {
            return Ok(()); // 새 라인 없음
        }

        // 마지막으로 읽은 라인 이후의 새로운 라인들만 추출
        let start_line = self.last_line_count;

        if start_line < lines.len() {
            let new_lines: Vec<String> = lines[start_line..]
                .iter()
                .map(|line| line.trim().trim_start_matches('\u{feff}').to_string())
                .collect();
            
            if !new_lines.is_empty() {
                if let Some(ref callback) = self.on_new_log_lines {
                    callback(new_lines);
                }
            }
        }

        // 현재 총 라인 수를 기억
        self.last_line_count = lines.len();
        self.last_position = file_content.len() as u64;
        Ok(())
    }

    async fn on_log_folder_change_event(&mut self) {
        let latest = self.find_latest_local_log().await;
        
        if let Some(latest_path) = latest {
            if self.log_file.as_ref() != Some(&latest_path) {
                println!("[INFO] Switching to new log file: {}", 
                    latest_path.file_name().unwrap_or_default().to_string_lossy());
                self.log_file = Some(latest_path.clone());
                
                // 파일 크기와 라인 카운트 재설정
                if let Ok(metadata) = tokio::fs::metadata(&latest_path).await {
                    self.last_position = metadata.len();
                }
                self.last_line_count = 0; // 새 파일이므로 라인 카운트 재설정
                
                if let Some(ref callback) = self.on_log_file_change {
                    callback();
                }
            }
        } else if self.log_file.is_some() {
            println!("[WARNING] Current log file '{}' is no longer the preferred log. No suitable new log found.",
                self.log_file.as_ref().unwrap().file_name().unwrap_or_default().to_string_lossy());
            self.log_file = None;
            self.last_position = 0;
            self.last_line_count = 0;
            
            if let Some(ref callback) = self.on_log_file_change {
                callback();
            }
        }
    }

    async fn monitor_loop(
        mut monitor: LogMonitor,
        mut stop_signal_receiver: mpsc::Receiver<()>,
    ) {
        println!("[INFO] 로그 모니터링 루프 시작.");
        
        loop {
            tokio::select! {
                _ = stop_signal_receiver.recv() => {
                    info!("정지 신호 수신, 로그 모니터링 루프 종료.");
                    break;
                },
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    // Python과 동일한 2초 간격
                    if monitor.log_file.is_none() {
                        monitor.log_file = monitor.find_latest_local_log().await;
                        if let Some(ref log_file) = monitor.log_file {
                            println!("[INFO] Monitoring log file: {} (full path: {})", 
                                log_file.file_name().unwrap_or_default().to_string_lossy(),
                                log_file.display());
                            
                            if let Ok(metadata) = tokio::fs::metadata(log_file).await {
                                monitor.last_position = metadata.len();
                            }
                            monitor.last_line_count = 0;
                        } else {
                            println!("[INFO] Waiting for a suitable log file...");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }

                    // 모니터링 상태 체크 (매 10회마다)
                    static mut MONITOR_COUNT: u32 = 0;
                    unsafe {
                        MONITOR_COUNT += 1;
                        
                        // 매 300회(10분)마다만 최신 로그 파일 재확인 (너무 자주 하지 않도록)
                        if MONITOR_COUNT % 300 == 0 {
                            // 현재 로그 파일이 여전히 존재하고 최신인지 간단히 확인
                            if let Some(ref current_file) = monitor.log_file {
                                if !current_file.exists() {
                                    println!("[INFO] Current log file no longer exists, searching for new one...");
                                    let latest = monitor.find_latest_local_log().await;
                                    if let Some(latest_path) = latest {
                                        monitor.log_file = Some(latest_path.clone());
                                        if let Ok(metadata) = tokio::fs::metadata(&latest_path).await {
                                            monitor.last_position = metadata.len();
                                        }
                                        monitor.last_line_count = 0;
                                    }
                                }
                            }
                        }
                    }

                    if let Err(e) = monitor.process_new_lines().await {
                        error!("로그 파일 처리 오류: {:?}", e);
                    }
                }
            }
        }
    }

    pub async fn start(&mut self, system_change_processor: Arc<Mutex<SystemChangeProcessor>>) -> Result<(), Box<dyn std::error::Error>> {
        self.monitoring = true;
        self.log_file = self.find_latest_local_log().await;
        
        if let Some(ref log_file) = self.log_file {
            if let Ok(metadata) = tokio::fs::metadata(log_file).await {
                self.last_position = metadata.len();
            }
        }

        if self.log_file.is_some() && !self.character_name.is_empty() {
            println!("[INFO] LogMonitor started for character: {}", self.character_name);

            let (stop_tx, stop_rx) = mpsc::channel(1);
            *self.stop_signal_sender.lock().await = Some(stop_tx);

            // Python과 동일한 파일 시스템 감시 설정
            // TODO: notify 설정 구현

            // 모니터링 태스크 시작 - 콜백 복사본 생성
            
            // 콜백을 take하지 말고 복사본으로 전달
            let mut monitor_clone = LogMonitor {
                config_manager: self.config_manager.clone(),
                log_processor: EveLogProcessor::new(self.logs_path.clone(), self.language.clone()), // 새 인스턴스
                logs_path: self.logs_path.clone(),
                character_name: self.character_name.clone(),
                language: self.language.clone(),
                log_file: self.log_file.clone(),
                last_position: self.last_position,
                last_line_count: self.last_line_count,
                monitoring: self.monitoring,
                observer: None,
                monitor_task: Arc::new(Mutex::new(None)),
                stop_signal_sender: Arc::new(Mutex::new(None)),
                on_new_log_lines: None, // 아래에서 설정
                on_log_file_change: None, // 아래에서 설정
            };
            
            // 콜백 설정
            let scp_clone = system_change_processor.clone();
            let callback = Box::new(move |lines: Vec<String>| {
                let scp = scp_clone.clone();
                tauri::async_runtime::spawn(async move {
                    for line in lines {
                        scp.lock().await.process_log_line(&line).await;
                    }
                });
            });
            monitor_clone.on_new_log_lines = Some(callback);

            let task = tokio::spawn(Self::monitor_loop(monitor_clone, stop_rx));
            *self.monitor_task.lock().await = Some(task);
        } else {
            println!("[ERROR] LogMonitor failed to initialize. No suitable log file found or character name not detected.");
            self.monitoring = false;
            return Err("LogMonitor failed to initialize".into());
        }

        Ok(())
    }

    pub async fn stop(&mut self) {
        self.monitoring = false;
        
        if let Some(sender) = self.stop_signal_sender.lock().await.take() {
            let _ = sender.send(()).await;
        }
        
        if let Some(task) = self.monitor_task.lock().await.take() {
            task.abort();
        }
        
        // TODO: observer 정리
    }
}