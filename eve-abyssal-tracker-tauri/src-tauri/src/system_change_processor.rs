use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
};
use chrono::{DateTime, Local, NaiveDateTime, Duration, TimeZone};
use serde::{Serialize, Deserialize};
use log::*;

use crate::eve_log_processor::EveLogProcessor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbyssalRunData {
    start: DateTime<Local>,
    end: DateTime<Local>,
    duration: Duration,
    duration_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub current_system: Option<String>,
    pub previous_system: Option<String>,
    pub last_updated: Option<DateTime<Local>>,
}

pub struct SystemChangeProcessor {
    log_processor: Arc<tokio::sync::Mutex<EveLogProcessor>>,
    on_abyssal_run_end: Option<Box<dyn Fn(DateTime<Local>, DateTime<Local>) + Send + Sync>>,
    
    abyssal_run_start: Option<DateTime<Local>>,
    abyssal_run_start_kst: Option<DateTime<Local>>,
    abyssal_run_count: u32,
    current_system: Option<String>,
    previous_system: Option<String>,
    last_system_change_time: Option<DateTime<Local>>,
    runs_by_date: HashMap<String, Vec<AbyssalRunData>>,
}

impl SystemChangeProcessor {
    pub fn new(
        log_processor: Arc<tokio::sync::Mutex<EveLogProcessor>>,
        on_abyssal_run_end: Option<Box<dyn Fn(DateTime<Local>, DateTime<Local>) + Send + Sync>>,
    ) -> Self {
        SystemChangeProcessor {
            log_processor,
            on_abyssal_run_end,
            abyssal_run_start: None,
            abyssal_run_start_kst: None,
            abyssal_run_count: 0,
            current_system: None,
            previous_system: None,
            last_system_change_time: None,
            runs_by_date: HashMap::new(),
        }
    }

    pub fn get_location_info(&self) -> LocationInfo {
        LocationInfo {
            current_system: self.current_system.clone(),
            previous_system: self.previous_system.clone(),
            last_updated: self.last_system_change_time,
        }
    }

    // 로그 파일을 스캔하여 최신 위치 정보를 업데이트하는 메서드
    pub async fn scan_latest_location(&mut self, character_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (files, language) = {
            let log_processor = self.log_processor.lock().await;
            let files = log_processor.find_all_log_files();
            let language = log_processor.language.clone();
            (files, language)
        };
        
        // 수정 시간으로 정렬 (최신 파일부터)
        let mut files_with_mtime: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        for file in files {
            if let Ok(metadata) = std::fs::metadata(&file) {
                if let Ok(mtime) = metadata.modified() {
                    files_with_mtime.push((file, mtime));
                }
            }
        }
        files_with_mtime.sort_by(|a, b| b.1.cmp(&a.1)); // 최신 순으로 정렬
        
        let mut recent_systems: Vec<(String, DateTime<Local>)> = Vec::new();
        
        // 최신 파일부터 확인하되, 충분한 위치 정보를 얻을 때까지만 스캔
        for (file, _) in files_with_mtime.iter().take(5) { // 최신 5개 파일만 스캔
            // 기존 log_processor를 사용하여 파일 설정
            {
                let mut log_processor = self.log_processor.lock().await;
                log_processor.set_log_file(file.clone());
                
                if !character_name.is_empty() {
                    let detected_name = log_processor.detect_character_name(Some(&file));
                    if detected_name.is_none() || detected_name.unwrap() != character_name {
                        continue;
                    }
                }
            }
            
            // 해당 파일의 모든 시스템 변경을 수집
            let mut file_systems: Vec<(String, DateTime<Local>)> = Vec::new();
            
            // 기존 log_processor를 사용하여 라인 읽기
            let lines = {
                let log_processor = self.log_processor.lock().await;
                log_processor.iter_lines(Some(&file)).collect::<Vec<_>>()
            };
            
            for line in lines {
                // 기존 process_log_line과 동일한 방식으로 처리
                let (is_system_change, parsed_data) = {
                    let log_processor = self.log_processor.lock().await;
                    if log_processor.is_system_change_line(&line) {
                        let (system_name, ts) = log_processor.parse_system_change(&line);
                        if system_name.is_some() && ts.is_some() {
                            let is_unknown = log_processor.is_unknown_system(system_name.as_ref().unwrap());
                            (true, Some((system_name.unwrap(), ts.unwrap(), is_unknown)))
                        } else {
                            (false, None)
                        }
                    } else {
                        (false, None)
                    }
                };
                
                if let Some((system_name, ts_str, is_unknown)) = parsed_data {
                    let event_time = match NaiveDateTime::parse_from_str(&ts_str, "%Y.%m.%d %H:%M:%S") {
                        Ok(naive_dt) => Local.from_local_datetime(&naive_dt).unwrap(),
                        Err(_) => continue,
                    };
                    
                    // 알 수 없는 시스템은 제외 (어비셜 데드스페이스)
                    if !is_unknown {
                        file_systems.push((system_name, event_time));
                    }
                }
            }
            
            // 파일 내에서 시간순으로 정렬 후 recent_systems에 추가
            file_systems.sort_by(|a, b| a.1.cmp(&b.1));
            recent_systems.extend(file_systems);
            
            // 충분한 데이터가 모였으면 중단 (최소 2개 이상의 위치가 필요)
            if recent_systems.len() >= 10 {
                break;
            }
        }
        
        // 전체 시스템 변경을 시간순으로 정렬
        recent_systems.sort_by(|a, b| a.1.cmp(&b.1));
        
        // 가장 최신의 2개 위치 설정
        if let Some((latest_system, latest_time)) = recent_systems.last() {
            self.current_system = Some(latest_system.clone());
            self.last_system_change_time = Some(*latest_time);
            
            // 이전 위치 찾기 (현재 위치와 다른 가장 최근 위치)
            self.previous_system = recent_systems.iter()
                .rev()
                .skip(1) // 현재 위치 건너뛰기
                .find(|(system, _)| system != latest_system)
                .map(|(system, _)| system.clone());
        }
        
        Ok(())
    }

    pub async fn process_log_line(&mut self, line: &str) {
        let (is_system_change, parsed_data) = {
            let log_processor = self.log_processor.lock().await;
            if log_processor.is_system_change_line(line) {
                let (system_name, ts) = log_processor.parse_system_change(line);
                if system_name.is_some() && ts.is_some() {
                    let is_unknown = log_processor.is_unknown_system(system_name.as_ref().unwrap());
                    (true, Some((system_name.unwrap(), ts.unwrap(), is_unknown)))
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        }; // lock 해제됨
        
        if !is_system_change {
            return;
        }
        
        if let Some((system_name, ts_str, is_unknown)) = parsed_data {
            // Python과 동일한 시간 파싱
            let event_time = match NaiveDateTime::parse_from_str(&ts_str, "%Y.%m.%d %H:%M:%S") {
                Ok(naive_dt) => naive_dt,
                Err(_) => return, // Python과 동일하게 파싱 실패 시 리턴
            };
            
            // Python과 동일한 시간대 처리 (UTC로 가정하고 KST로 변환)
            let event_time_local = Local.from_local_datetime(&event_time).unwrap();
            let event_time_kst = event_time_local + Duration::hours(9);
            
            if is_unknown {
                if self.abyssal_run_start.is_none() {
                    self.abyssal_run_start = Some(event_time_local);
                    self.abyssal_run_start_kst = Some(event_time_kst);
                    info!("[START] Abyssal Deadspace entered at {} (KST)", 
                        self.abyssal_run_start_kst.unwrap().format("%Y-%m-%d %H:%M:%S"));
                }
            } else {
                if let Some(start_time) = self.abyssal_run_start {
                    let end_time = event_time_local;
                    let end_time_kst = event_time_kst;
                    let duration = end_time - start_time;
                    let mins = duration.num_minutes();
                    let secs = duration.num_seconds() % 60;
                    self.abyssal_run_count += 1;
                    
                    info!("[END] Returned to normal space at {} (KST). Run duration: {}m {}s. Total runs: {}", 
                        end_time_kst.format("%Y-%m-%d %H:%M:%S"), mins, secs, self.abyssal_run_count);
                    
                    if let Some(ref callback) = self.on_abyssal_run_end {
                        callback(self.abyssal_run_start_kst.unwrap(), end_time_kst);
                    }
                    
                    self.abyssal_run_start = None;
                    self.abyssal_run_start_kst = None;
                }
            }
            
            if self.current_system.is_some() && self.current_system.as_ref() != Some(&system_name) {
                self.previous_system = self.current_system.clone();
            }
            
            self.current_system = Some(system_name);
            self.last_system_change_time = Some(event_time_local);
        }
    }

    pub async fn scan_past_runs(&mut self, logs_path: &str, character_name: &str) {
        let log_processor = self.log_processor.lock().await;
        let files = log_processor.find_all_log_files();
        drop(log_processor);
        
        // Python과 동일하게 수정 시간으로 정렬
        let mut files_with_mtime: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        for file in files {
            if let Ok(metadata) = std::fs::metadata(&file) {
                if let Ok(mtime) = metadata.modified() {
                    files_with_mtime.push((file, mtime));
                }
            }
        }
        files_with_mtime.sort_by(|a, b| a.1.cmp(&b.1)); // 오래된 순으로 정렬 (Python과 동일)
        
        info!("[INFO] Scanning {} past log files for runs...", files_with_mtime.len());
        
        // 루프 밖에서 한 번만 language 가져오기
        let language = {
            let log_processor = self.log_processor.lock().await;
            log_processor.language.clone()
        };
        
        for (file, _) in files_with_mtime {
            let logs_path_buf = PathBuf::from(logs_path);
            
            let mut temp_log_processor = EveLogProcessor::new(logs_path_buf, language.clone());
            temp_log_processor.set_log_file(file.clone());
            
            if !character_name.is_empty() {
                let detected_name = temp_log_processor.detect_character_name(Some(&file));
                if detected_name.is_none() || detected_name.unwrap() != character_name {
                    continue;
                }
            }
            
            let mut abyssal_run_start: Option<DateTime<Local>> = None;
            
            for line in temp_log_processor.iter_lines(Some(&file)) {
                if temp_log_processor.is_system_change_line(&line) {
                    let (system_name, ts) = temp_log_processor.parse_system_change(&line);
                    
                    if system_name.is_none() || ts.is_none() {
                        continue;
                    }
                    
                    let system_name = system_name.unwrap();
                    let ts_str = ts.unwrap();
                    
                    let event_time = match NaiveDateTime::parse_from_str(&ts_str, "%Y.%m.%d %H:%M:%S") {
                        Ok(naive_dt) => Local.from_local_datetime(&naive_dt).unwrap(),
                        Err(_) => continue,
                    };
                    
                    if temp_log_processor.is_unknown_system(&system_name) {
                        if abyssal_run_start.is_none() {
                            abyssal_run_start = Some(event_time);
                        }
                    } else {
                        if let Some(start_time) = abyssal_run_start {
                            let end_time = event_time;
                            let duration = end_time - start_time;
                            let mins = duration.num_minutes();
                            let secs = duration.num_seconds() % 60;
                            
                            let date_str = start_time.format("%Y-%m-%d").to_string();
                            let run_data = AbyssalRunData {
                                start: start_time,
                                end: end_time,
                                duration,
                                duration_str: format!("{}m {}s", mins, secs),
                            };
                            
                            self.runs_by_date.entry(date_str).or_insert_with(Vec::new).push(run_data);
                            abyssal_run_start = None;
                        }
                    }
                }
            }
        }
    }

    pub fn print_past_runs(&self) {
        if !self.runs_by_date.is_empty() {
            info!("[PAST RUNS]");
            let mut sorted_dates: Vec<_> = self.runs_by_date.keys().collect();
            sorted_dates.sort();
            
            for date in sorted_dates {
                if let Some(runs) = self.runs_by_date.get(date) {
                    info!("{}:", date);
                    for run in runs {
                        let start_kst = run.start + Duration::hours(9);
                        let end_kst = run.end + Duration::hours(9);
                        let start_str = start_kst.format("%H:%M:%S");
                        let end_str = end_kst.format("%H:%M:%S");
                        info!("  - {} ~ {} ({}) (KST)", start_str, end_str, run.duration_str);
                    }
                }
            }
        } else {
            info!("[PAST RUNS] None");
        }
    }
}

