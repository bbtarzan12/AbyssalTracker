use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use std::fs;
use polars::prelude::*;
use regex::Regex;
use chrono::{DateTime, Local};

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
        // data 디렉토리만 사용 (Python과 일치)
        let data_dir_path = PathBuf::from("data");

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
        println!("[DEBUG] save_abyssal_result - raw items: '{}'", items);
        
        if items.is_empty() {
            println!("[DEBUG] Items empty, not saving");
            return Ok(()); // Python과 동일: 아이템이 없으면 저장하지 않음
        }

        // 아이템 파싱 테스트
        let parsed_items = self.parse_items(items);
        println!("[DEBUG] Parsed {} items", parsed_items.len());

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

            println!("[DEBUG] Existing DataFrame schema:");
            for col in existing_df.get_column_names() {
                if let Ok(series) = existing_df.column(col) {
                    println!("  {}: {:?}", col, series.dtype());
                }
            }
            
            println!("[DEBUG] New DataFrame schema:");
            for col in new_row_df.get_column_names() {
                if let Ok(series) = new_row_df.column(col) {
                    println!("  {}: {:?}", col, series.dtype());
                }
            }

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
                        println!("[DEBUG] Unexpected seconds column type: {:?}", existing_seconds_col.dtype());
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
            
            println!("[DEBUG] CSV file updated with {} rows", existing_df.height());
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
                
            println!("[DEBUG] New CSV file created with {} rows", df.height());
        }
        
        println!("[DEBUG] save_abyssal_result completed successfully");
        Ok(())
    }

    pub fn parse_items(&self, item_str: &str) -> Vec<(String, i32)> {
        println!("[DEBUG] parse_items input: '{}'", item_str);
        
        // Python과 정확히 동일한 로직, 탭과 세미콜론 모두 구분자로 처리
        let regex = Regex::new(r"(.+?)\*\s*(\d+)?").unwrap(); // $ 제거
        let mut items = Vec::new();
        
        // 먼저 '*탭숫자' 패턴을 '*공백숫자'로 변환
        let star_tab_regex = Regex::new(r"\*\t(\d+)").unwrap();
        let step1 = star_tab_regex.replace_all(item_str, "* $1");
        println!("[DEBUG] after step1: '{}'", step1);
        
        // 그 다음 나머지 탭을 세미콜론으로 치환하고, 연속된 세미콜론들을 하나로 합침
        let normalized_str = step1
            .replace('\t', ";")
            .replace(";;", ";")
            .replace("; ;", ";");
        println!("[DEBUG] normalized_str: '{}'", normalized_str);
        
        for (i, entry) in normalized_str.split(';').enumerate() {
            let entry = entry.trim();
            println!("[DEBUG] processing entry {}: '{}'", i, entry);
            
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
                println!("[DEBUG] regex match - name: '{}', qty: {}", clean_name, qty);
                items.push((clean_name, qty));
            } else {
                // '*' 제거 후 추가
                let clean_entry = entry.replace('*', "").trim().to_string();
                println!("[DEBUG] no regex match - clean_entry: '{}'", clean_entry);
                items.push((clean_entry, 1));
            }
        }
        
        println!("[DEBUG] final parsed items: {:?}", items);
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
}