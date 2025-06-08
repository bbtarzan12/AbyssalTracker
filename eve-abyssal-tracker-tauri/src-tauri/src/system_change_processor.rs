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

pub struct SystemChangeProcessor {
    log_processor: Arc<tokio::sync::Mutex<EveLogProcessor>>,
    on_abyssal_run_end: Option<Box<dyn Fn(DateTime<Local>, DateTime<Local>) + Send + Sync>>,
    
    abyssal_run_start: Option<DateTime<Local>>,
    abyssal_run_start_kst: Option<DateTime<Local>>,
    abyssal_run_count: u32,
    current_system: Option<String>,
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
            runs_by_date: HashMap::new(),
        }
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
            self.current_system = Some(system_name);
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

