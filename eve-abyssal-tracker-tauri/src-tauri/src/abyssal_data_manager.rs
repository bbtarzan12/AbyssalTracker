use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use std::fs;
use polars::prelude::*;
use regex::Regex;
use chrono::{DateTime, Local};
use std::ops::{BitAnd, Not};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AbyssalResult {
    #[serde(rename = "시작시각(KST)")]
    pub start_time_kst: String,
    #[serde(rename = "종료시각(KST)")]
    pub end_time_kst: String,
    #[serde(rename = "런 소요(초)")]
    pub run_time_seconds: f64,
    #[serde(rename = "런 소요(분)")]
    pub run_time_minutes: f64,
    #[serde(rename = "어비셜 종류")]
    pub abyssal_type: String,
    #[serde(rename = "획득 아이템")]
    pub acquired_items: String,
}

#[derive(Clone)]
pub struct AbyssalDataManager {
    app_handle: AppHandle,
    data_dir_path: PathBuf,
}

impl AbyssalDataManager {
    pub fn new(app_handle: AppHandle) -> Self {
        // 앱 데이터 디렉토리 사용 (설치된 앱에서 안전한 위치)
        let data_dir_path = match app_handle.path().app_data_dir() {
            Ok(app_data_dir) => {
                let data_dir = app_data_dir.join("data");
                // 디렉토리가 없으면 생성
                if let Err(e) = std::fs::create_dir_all(&data_dir) {
                    eprintln!("Warning: Failed to create app data directory: {}", e);
                    // 실패 시 현재 디렉토리의 data 폴더 사용
                    PathBuf::from("data")
                } else {
                    data_dir
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to get app data directory: {}, using local data directory", e);
                PathBuf::from("data")
            }
        };

        AbyssalDataManager {
            app_handle,
            data_dir_path,
        }
    }

    pub fn load_abyssal_results(&self) -> Result<DataFrame, String> {
        if !self.data_dir_path.exists() {
            return Err("Data directory does not exist".to_string());
        }

        let mut all_dataframes = Vec::new();

        // Read all CSV files that match the pattern abyssal_results_*.csv
        let entries = fs::read_dir(&self.data_dir_path)
            .map_err(|e| format!("Failed to read data directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("abyssal_results_") && file_name.ends_with(".csv") {
                    match CsvReader::from_path(&path)
                        .map_err(|e| format!("Failed to open data file {}: {}", file_name, e))?
                        .has_header(true)
                        .finish()
                    {
                        Ok(df) => all_dataframes.push(df),
                        Err(e) => eprintln!("Warning: Failed to read CSV {}: {}", file_name, e),
                    }
                }
            }
        }

        if all_dataframes.is_empty() {
            // Create an empty DataFrame with the expected schema if no files exist
            return Ok(DataFrame::new(vec![
                Series::new("시작시각(KST)", Vec::<String>::new()),
                Series::new("종료시각(KST)", Vec::<String>::new()),
                Series::new("런 소요(초)", Vec::<f64>::new()),
                Series::new("런 소요(분)", Vec::<f64>::new()),
                Series::new("어비셜 종류", Vec::<String>::new()),
                Series::new("획득 아이템", Vec::<String>::new()),
            ]).map_err(|e| format!("Failed to create empty DataFrame: {}", e))?);
        }

        // Concatenate all dataframes
        let combined_df = all_dataframes.into_iter().reduce(|acc, df| {
            acc.vstack(&df).unwrap_or(acc)
        }).unwrap();

        Ok(combined_df)
    }

    pub fn save_abyssal_result(&self, start_time: DateTime<Local>, end_time: DateTime<Local>, acquired_items: String, abyssal_type: String) -> Result<(), String> {
        let items = acquired_items.trim();
        
        if items.is_empty() {
            return Ok(()); // Python과 동일: 아이템이 없으면 저장하지 않음
        }

        // 아이템 파싱 테스트
        let parsed_items = self.parse_items(items);

        // data 디렉토리 생성
        fs::create_dir_all(&self.data_dir_path)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        // 날짜별 파일명 생성 (Python과 일치)
        let date_str = start_time.format("%Y-%m-%d").to_string();
        let filename = format!("abyssal_results_{}.csv", date_str);
        let data_file_path = self.data_dir_path.join(&filename);
        
        let file_exists = data_file_path.exists();
        
        // 지속시간 계산 (Python과 일치)
        let duration = end_time - start_time;
        let duration_sec = duration.num_seconds() as f64;
        let duration_min = (duration_sec / 60.0 * 100.0).round() / 100.0; // 소수점 2자리 반올림
        
        // 아이템 문자열 정규화 (Python과 일치)
        let items = items.replace('\n', "; ").replace('\r', "");
        
        // 새 행 데이터 생성 (Python과 정확히 일치)
        let new_row_df = DataFrame::new(vec![
            Series::new("시작시각(KST)", &[start_time.format("%Y-%m-%d %H:%M:%S").to_string()]),
            Series::new("종료시각(KST)", &[end_time.format("%Y-%m-%d %H:%M:%S").to_string()]),
            Series::new("런 소요(초)", &[duration_sec as i64]),
            Series::new("런 소요(분)", &[duration_min]),
            Series::new("어비셜 종류", &[abyssal_type.clone()]),
            Series::new("획득 아이템", &[items.clone()]),
        ]).map_err(|e| format!("Failed to create new row DataFrame: {}", e))?;

        if file_exists {
            // 기존 파일에 추가
            let mut existing_df = CsvReader::from_path(&data_file_path)
                .map_err(|e| format!("Failed to open existing data file: {}", e))?
                .has_header(true)
                .finish()
                .map_err(|e| format!("Failed to read existing CSV: {}", e))?;

            // 기존 파일의 스키마와 맞추기 위해 타입 변환
            let new_row_df = if let Ok(existing_seconds_col) = existing_df.column("런 소요(초)") {
                match existing_seconds_col.dtype() {
                    polars::datatypes::DataType::Int64 => {
                        // 이미 i64로 되어 있으면 그대로
                        new_row_df
                    },
                    polars::datatypes::DataType::Float64 => {
                        // 기존이 Float64면 새 데이터도 Float64로
                        DataFrame::new(vec![
                            Series::new("시작시각(KST)", &[start_time.format("%Y-%m-%d %H:%M:%S").to_string()]),
                            Series::new("종료시각(KST)", &[end_time.format("%Y-%m-%d %H:%M:%S").to_string()]),
                            Series::new("런 소요(초)", &[duration_sec]),  // Float64로
                            Series::new("런 소요(분)", &[duration_min]),
                            Series::new("어비셜 종류", &[abyssal_type.clone()]),
                            Series::new("획득 아이템", &[items.clone()]),
                        ]).map_err(|e| format!("Failed to create Float64 DataFrame: {}", e))?
                    },
                    _ => {
                        new_row_df
                    }
                }
            } else {
                new_row_df
            };

            existing_df = existing_df.vstack(&new_row_df)
                .map_err(|e| format!("Failed to append new row: {}", e))?;

            // UTF-8-BOM으로 저장 (Python과 일치)
            let mut file = fs::File::create(&data_file_path)
                .map_err(|e| format!("Failed to create data file: {}", e))?;
            
            // UTF-8 BOM 작성
            use std::io::Write;
            file.write_all(&[0xEF, 0xBB, 0xBF])
                .map_err(|e| format!("Failed to write BOM: {}", e))?;

            CsvWriter::new(file)
                .finish(&mut existing_df)
                .map_err(|e| format!("Failed to write CSV: {}", e))?;
        } else {
            // 새 파일 생성
            let mut df = new_row_df;
            let mut file = fs::File::create(&data_file_path)
                .map_err(|e| format!("Failed to create data file: {}", e))?;

            // UTF-8 BOM 작성
            use std::io::Write;
            file.write_all(&[0xEF, 0xBB, 0xBF])
                .map_err(|e| format!("Failed to write BOM: {}", e))?;

            CsvWriter::new(file)
                .finish(&mut df)
                .map_err(|e| format!("Failed to write CSV: {}", e))?;
        }
        
        Ok(())
    }

    pub fn parse_items(&self, item_str: &str) -> Vec<(String, i32)> {
        // Python과 정확히 동일한 로직, 탭과 세미콜론 모두 구분자로 처리
        let regex = Regex::new(r"(.+?)\*\s*(\d+)?").unwrap(); // $ 제거
        let mut items = Vec::new();
        
        // 먼저 '*탭숫자' 패턴을 '*공백숫자'로 변환
        let star_tab_regex = Regex::new(r"\*\t(\d+)").unwrap();
        let step1 = star_tab_regex.replace_all(item_str, "* $1");
        
        // 그 다음 나머지 탭을 세미콜론으로 치환하고, 연속된 세미콜론들을 하나로 합침
        let normalized_str = step1
            .replace('\t', ";")
            .replace(";;", ";")
            .replace("; ;", ";");
        
        for entry in normalized_str.split(';') {
            let entry = entry.trim();
            
            if entry.is_empty() {
                continue;
            }
            
            if let Some(captures) = regex.captures(entry) {
                let name = captures.get(1).unwrap().as_str().trim().to_string();
                // 아이템 이름에서 '*' 제거 (ESI API용)
                let clean_name = name.replace('*', "").trim().to_string();
                let qty = if let Some(qty_match) = captures.get(2) {
                    qty_match.as_str().parse::<i32>().unwrap_or(1)
                } else {
                    1
                };
                items.push((clean_name, qty));
            } else {
                // '*' 제거 후 추가
                let clean_entry = entry.replace('*', "").trim().to_string();
                items.push((clean_entry, 1));
            }
        }
        
        items
    }

    pub fn abyssal_type_to_filament_name(&self, abyssal_type: &str) -> Option<String> {
        // Python과 정확히 동일한 로직
        let tier_map = [
            ("T1", "Calm"),
            ("T2", "Agitated"), 
            ("T3", "Fierce"),
            ("T4", "Raging"),
            ("T5", "Chaotic"),
            ("T6", "Cataclysmic"),
        ];
        
        let parts: Vec<&str> = abyssal_type.split_whitespace().collect();
        if parts.len() >= 2 {
            let tier = parts[0];
            let weather = parts[1];
            
            for &(tier_key, tier_name) in &tier_map {
                if tier == tier_key {
                    return Some(format!("{} {} Filament", tier_name, weather));
                }
            }
        }
        None
    }

    pub fn delete_abyssal_run(&self, start_time_kst: &str, end_time_kst: &str) -> Result<(), String> {
        // 시작 시간에서 날짜 추출
        let date_str = if let Some(date_part) = start_time_kst.split(' ').next() {
            date_part.to_string()
        } else {
            return Err("Invalid start time format".to_string());
        };
        
        let filename = format!("abyssal_results_{}.csv", date_str);
        let data_file_path = self.data_dir_path.join(&filename);
        
        if !data_file_path.exists() {
            return Err(format!("Data file not found: {}", filename));
        }
        
        // CSV 파일 읽기
        let mut df = CsvReader::from_path(&data_file_path)
            .map_err(|e| format!("Failed to open data file: {}", e))?
            .has_header(true)
            .finish()
            .map_err(|e| format!("Failed to read CSV: {}", e))?;
        
        // 삭제할 행 찾기 (시작시간과 종료시간이 모두 일치하는 행)
        let start_time_col = df.column("시작시각(KST)")
            .map_err(|e| format!("Failed to get start time column: {}", e))?;
        let end_time_col = df.column("종료시각(KST)")
            .map_err(|e| format!("Failed to get end time column: {}", e))?;
        
        // 조건에 맞지 않는 행들만 유지 (즉, 삭제할 행 제외)
        let mask = start_time_col.equal(start_time_kst)
            .map_err(|e| format!("Failed to create start time mask: {}", e))?
            .bitand(
                end_time_col.equal(end_time_kst)
                    .map_err(|e| format!("Failed to create end time mask: {}", e))?
            )
            .not(); // 조건에 맞는 행을 제외하기 위해 not() 적용
        
        let filtered_df = df.filter(&mask)
            .map_err(|e| format!("Failed to filter DataFrame: {}", e))?;
        
        if filtered_df.height() == df.height() {
            return Err("Run not found in the data file".to_string());
        }
        
        // 필터링된 데이터를 다시 저장
        if filtered_df.height() == 0 {
            // 모든 행이 삭제되었으면 파일 삭제
            fs::remove_file(&data_file_path)
                .map_err(|e| format!("Failed to delete empty data file: {}", e))?;
        } else {
            // 남은 데이터를 파일에 저장
            let mut df_to_write = filtered_df;
            
            // UTF-8-BOM으로 저장
            let mut file = fs::File::create(&data_file_path)
                .map_err(|e| format!("Failed to create data file: {}", e))?;
            
            use std::io::Write;
            file.write_all(&[0xEF, 0xBB, 0xBF])
                .map_err(|e| format!("Failed to write BOM: {}", e))?;

            CsvWriter::new(file)
                .finish(&mut df_to_write)
                .map_err(|e| format!("Failed to write CSV: {}", e))?;
        }
        
        Ok(())
    }
}