use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use polars::prelude::*;
use tauri::{AppHandle, Emitter};
use crate::{eve_api::EVEApi, abyssal_data_manager::AbyssalDataManager};

// Implement From<String> for anyhow::Error to allow using `?` with String errors

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunData {
    #[serde(rename = "시작시각(KST)")]
    pub start_time: String,
    #[serde(rename = "종료시각(KST)")]
    pub end_time: String,
    #[serde(rename = "런 소요(분)")]
    pub run_time_minutes: f64,
    #[serde(rename = "어비셜 종류")]
    pub abyssal_type: String,
    #[serde(rename = "실수익")]
    pub net_profit: f64,
    #[serde(rename = "ISK/h")]
    pub isk_per_hour: f64,
    #[serde(rename = "획득 아이템")]
    pub acquired_items: String,
    #[serde(rename = "날짜")]
    pub date: String,
    #[serde(rename = "드롭")]
    pub drop_value: f64,
    #[serde(rename = "입장료")]
    pub entry_cost: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyStats {
    pub runs: Vec<RunData>,
    pub avg_isk: f64,
    pub avg_time: f64,
    pub avg_iskph: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TierWeatherStats {
    pub tier: String,
    pub weather: String,
    pub runs_count: usize,
    pub avg_isk: f64,
    pub avg_time: f64,
    pub avg_iskph: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OverallStats {
    pub avg_isk: f64,
    pub avg_time: f64,
    pub avg_iskph: f64,
    pub tier_weather_stats: Vec<TierWeatherStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub df: Vec<RunData>,
    pub daily_stats: HashMap<String, DailyStats>,
    pub overall_stats: OverallStats,
    pub item_buy_price_cache: HashMap<String, f64>,
}

pub struct AbyssalDataAnalyzer {
    eve_api: Arc<EVEApi>,
    data_manager: Arc<Mutex<AbyssalDataManager>>,
    pub app_handle: Option<AppHandle>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LoadingProgress {
    pub step: String,
    pub message: String,
    pub progress: Option<f64>,
    pub completed: bool,
}

impl AbyssalDataAnalyzer {
    pub fn new(eve_api: Arc<EVEApi>, data_manager: Arc<Mutex<AbyssalDataManager>>) -> Self {
        Self {
            eve_api,
            data_manager,
            app_handle: None,
        }
    }

    pub fn with_app_handle(mut self, app_handle: AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    fn emit_progress(&self, step: &str, message: &str, progress: Option<f64>, completed: bool) {
        if let Some(ref app_handle) = self.app_handle {
            let progress_data = LoadingProgress {
                step: step.to_string(),
                message: message.to_string(),
                progress,
                completed,
            };
            let _ = app_handle.emit("loading_progress", progress_data);
        }
    }

    pub async fn analyze_data(&self) -> Result<AnalysisResult, anyhow::Error> {
        let start_total = std::time::Instant::now();
        
        self.emit_progress("csv_load", "CSV 파일 로드 중...", Some(0.0), false);
        println!("📂 [AbyssalDataAnalyzer] CSV 파일 로드 중...");
        let start_csv_load = std::time::Instant::now();
        
        let df = self.data_manager.lock().await.load_abyssal_results()
            .map_err(|e| anyhow::anyhow!("Failed to load abyssal results: {}", e))?;
        
        let end_csv_load = start_csv_load.elapsed();
        self.emit_progress("csv_load", &format!("CSV 파일 로드 완료 ({:.2}초)", end_csv_load.as_secs_f64()), Some(100.0), true);
        println!("  ▶️ CSV 파일 로드 완료. 소요 시간: {:.2}초 ✅", end_csv_load.as_secs_f64());
        
        if df.is_empty() {
            self.emit_progress("csv_load", "분석할 데이터가 없습니다", Some(100.0), true);
            println!("❌ 분석할 데이터가 없습니다.");
            return Ok(AnalysisResult {
                df: vec![],
                daily_stats: HashMap::new(),
                overall_stats: OverallStats {
                    avg_isk: 0.0,
                    avg_time: 0.0,
                    avg_iskph: 0.0,
                    tier_weather_stats: vec![],
                },
                item_buy_price_cache: HashMap::new(),
            });
        }

        println!("  ▶️ 총 {}개의 런 데이터 로드 완료. ✅", df.height());
        
        self.emit_progress("item_collection", "모든 아이템 이름 수집 중...", Some(0.0), false);
        println!("  ▶️ 모든 아이템 이름 수집 중... 🔍");
        let start_item_collection = std::time::Instant::now();
        
        // 모든 아이템 이름 수집 (Python과 동일한 로직)
        let mut all_item_names = std::collections::HashSet::new();
        
        // 획득 아이템에서 아이템 이름 추출
        if let Ok(items_column) = df.column("획득 아이템") {
            for item_str in items_column.str().unwrap().into_iter() {
                if let Some(items) = item_str {
                    let parsed_items = self.data_manager.lock().await.parse_items(items);
                    for (name, _qty) in parsed_items {
                        all_item_names.insert(name);
                    }
                }
            }
        }
        
        // 어비셜 종류에서 필라멘트 이름 추출
        if let Ok(abyssal_types_column) = df.column("어비셜 종류") {
            for abyssal_type in abyssal_types_column.str().unwrap().into_iter() {
                if let Some(abyssal_type_str) = abyssal_type {
                    if let Some(filament) = self.data_manager.lock().await.abyssal_type_to_filament_name(abyssal_type_str) {
                        all_item_names.insert(filament);
                    }
                }
            }
        }
        
        let all_item_names: Vec<String> = all_item_names.into_iter().collect();
        let end_item_collection = start_item_collection.elapsed();
        self.emit_progress("item_collection", &format!("{}종의 아이템 발견! ({:.2}초)", all_item_names.len(), end_item_collection.as_secs_f64()), Some(100.0), true);
        println!("  ▶️ {}종의 아이템 발견! 소요 시간: {:.2}초 ✨", all_item_names.len(), end_item_collection.as_secs_f64());

        self.emit_progress("type_id_fetch", "ESI API로 아이템 type_id 변환 중...", Some(0.0), false);
        println!("  ▶️ ESI API로 아이템 type_id 변환 중... 🔄");
        let start_type_id_fetch = std::time::Instant::now();
        let name_to_id = self.eve_api.fetch_type_ids(all_item_names.clone()).await?;
        let end_type_id_fetch = start_type_id_fetch.elapsed();
        self.emit_progress("type_id_fetch", &format!("{}종 변환 성공! (미매칭: {}) ({:.2}초)", name_to_id.len(), all_item_names.len() - name_to_id.len(), end_type_id_fetch.as_secs_f64()), Some(100.0), true);
        println!("  ▶️ {}종 변환 성공! (미매칭: {}) 소요 시간: {:.2}초 💡", 
            name_to_id.len(), all_item_names.len() - name_to_id.len(), end_type_id_fetch.as_secs_f64());

        self.emit_progress("price_fetch", "Fuzzwork로 대량 시세 조회 중...", Some(0.0), false);
        println!("  ▶️ Fuzzwork로 대량 시세 조회 중... 💰");
        let start_price_fetch = std::time::Instant::now();
        let ids: Vec<u32> = name_to_id.values().cloned().collect();
        let mut prices = HashMap::new();
        
        // 100개씩 청크로 나누어 처리 (Python과 동일)
        let total_chunks = (ids.len() + 99) / 100; // 올림 계산
        for (chunk_index, chunk) in ids.chunks(100).enumerate() {
            let progress = ((chunk_index as f64 / total_chunks as f64) * 100.0).min(99.0);
            self.emit_progress("price_fetch", &format!("시세 조회 중... ({}/{})", chunk_index + 1, total_chunks), Some(progress), false);
            
            let chunk_prices = self.eve_api.fetch_fuzzwork_prices(chunk.to_vec()).await?;
            prices.extend(chunk_prices);
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        
        let end_price_fetch = start_price_fetch.elapsed();
        self.emit_progress("price_fetch", &format!("Fuzzwork 시세 조회 완료 ({:.2}초)", end_price_fetch.as_secs_f64()), Some(100.0), true);
        println!("  ▶️ Fuzzwork 시세 조회 완료. 소요 시간: {:.2}초 💸", end_price_fetch.as_secs_f64());

        // 가격 캐시 생성 (Python과 동일한 로직)
        let mut item_buy_price_cache = HashMap::new();
        let mut item_sell_price_cache = HashMap::new();
        
        for (name, type_id) in &name_to_id {
            if let Some(price_data) = prices.get(&type_id.to_string()) {
                // buy.max 가격 추출 (문자열로 반환되므로 parse 필요)
                if let Some(buy_max_str) = price_data.get("buy")
                    .and_then(|buy| buy.get("max"))
                    .and_then(|max| max.as_str()) {
                    match buy_max_str.parse::<f64>() {
                        Ok(buy_max) => {
                            item_buy_price_cache.insert(name.clone(), buy_max);
                        },
                        Err(_) => {
                            item_buy_price_cache.insert(name.clone(), 0.0);
                            println!("[WARNING] 아이템 '{}'의 구매 가격 파싱 오류. 0으로 설정됨. 값: '{}'", name, buy_max_str);
                        }
                    }
                } else {
                    item_buy_price_cache.insert(name.clone(), 0.0);
                    println!("[WARNING] 아이템 '{}'의 구매 가격 데이터 없음. 0으로 설정됨.", name);
                }
                
                // sell.min 가격 추출 (문자열로 반환되므로 parse 필요)
                if let Some(sell_min_str) = price_data.get("sell")
                    .and_then(|sell| sell.get("min"))
                    .and_then(|min| min.as_str()) {
                    match sell_min_str.parse::<f64>() {
                        Ok(sell_min) => {
                            item_sell_price_cache.insert(name.clone(), sell_min);
                        },
                        Err(_) => {
                            item_sell_price_cache.insert(name.clone(), 0.0);
                            println!("[WARNING] 아이템 '{}'의 판매 가격 파싱 오류. 0으로 설정됨. 값: '{}'", name, sell_min_str);
                        }
                    }
                } else {
                    item_sell_price_cache.insert(name.clone(), 0.0);
                    println!("[WARNING] 아이템 '{}'의 판매 가격 데이터 없음. 0으로 설정됨.", name);
                }
            } else {
                item_buy_price_cache.insert(name.clone(), 0.0);
                item_sell_price_cache.insert(name.clone(), 0.0);
                println!("[WARNING] 아이템 '{}' ({})에 대한 가격 데이터 없음. 0으로 설정됨.", name, type_id);
            }
        }

        self.emit_progress("analysis", "런 지표 계산 및 통계 생성 중...", Some(0.0), false);
        println!("  ▶️ 런 지표 계산 및 통계 생성 중... 📊");
        let start_analysis = std::time::Instant::now();
        
        // 데이터를 먼저 벡터로 변환하여 계산을 단순화
        let mut runs_data = Vec::new();
        
        // DataFrame을 row별로 처리
        let start_time_col = df.column("시작시각(KST)").map_err(|e| anyhow::anyhow!("시작시각(KST) 컬럼 없음: {}", e))?.str().unwrap();
        let end_time_col = df.column("종료시각(KST)").map_err(|e| anyhow::anyhow!("종료시각(KST) 컬럼 없음: {}", e))?.str().unwrap();
        let run_time_col = df.column("런 소요(분)").map_err(|e| anyhow::anyhow!("런 소요(분) 컬럼 없음: {}", e))?.f64().unwrap();
        let abyssal_type_col = df.column("어비셜 종류").map_err(|e| anyhow::anyhow!("어비셜 종류 컬럼 없음: {}", e))?.str().unwrap();
        let acquired_items_col = df.column("획득 아이템").map_err(|e| anyhow::anyhow!("획득 아이템 컬럼 없음: {}", e))?.str().unwrap();
        
        let data_manager = self.data_manager.lock().await;
        
        for i in 0..df.height() {
            let start_time = start_time_col.get(i).unwrap_or("").to_string();
            let end_time = end_time_col.get(i).unwrap_or("").to_string();
            let run_time_minutes = run_time_col.get(i).unwrap_or(0.0);
            let abyssal_type = abyssal_type_col.get(i).unwrap_or("").to_string();
            let acquired_items = acquired_items_col.get(i).unwrap_or("").to_string();
            
            // 드롭 가격 계산
            let drop_value: f64 = {
                let parsed_items = data_manager.parse_items(&acquired_items);
                let total_drop_value: f64 = parsed_items.into_iter().map(|(name, qty)| {
                    let price = item_buy_price_cache.get(&name).unwrap_or(&0.0);
                    let item_value = price * (qty as f64);
                    item_value
                }).sum();
                total_drop_value
            };
            
            // 입장료 계산 (Python과 동일하게 sell.min 사용)
            let entry_cost: f64 = {
                if let Some(filament) = data_manager.abyssal_type_to_filament_name(&abyssal_type) {
                    let price = item_sell_price_cache.get(&filament).unwrap_or(&0.0);
                    let cost = price * 3.0; // 프리깃 3배
                    cost
                } else {
                    0.0
                }
            };
            
            // 실수익 및 ISK/h 계산
            let net_profit = drop_value - entry_cost;
            
            // 실수익 및 ISK/h 계산
            let isk_per_hour = if run_time_minutes > 0.0 {
                net_profit / (run_time_minutes / 60.0)
            } else {
                0.0
            };
            
            // 날짜 추출
            let date = if start_time.len() >= 10 {
                start_time[0..10].to_string()
            } else {
                "".to_string()
            };
            
            runs_data.push(RunData {
                start_time,
                end_time,
                run_time_minutes,
                abyssal_type,
                net_profit,
                isk_per_hour,
                acquired_items,
                date,
                drop_value,
                entry_cost,
            });
        }
        
        drop(data_manager); // Mutex 락 해제

        // 일별 통계 생성
        let mut daily_stats = HashMap::new();
        let mut grouped_by_date: HashMap<String, Vec<&RunData>> = HashMap::new();
        
        for run in &runs_data {
            grouped_by_date.entry(run.date.clone()).or_insert_with(Vec::new).push(run);
        }

        for (date, runs) in grouped_by_date {
            let avg_isk = if runs.is_empty() { 0.0 } else { 
                runs.iter().map(|r| r.net_profit).sum::<f64>() / runs.len() as f64 
            };
            let avg_time = if runs.is_empty() { 0.0 } else { 
                runs.iter().map(|r| r.run_time_minutes).sum::<f64>() / runs.len() as f64 
            };
            let avg_iskph = if runs.is_empty() { 0.0 } else { 
                runs.iter().map(|r| r.isk_per_hour).sum::<f64>() / runs.len() as f64 
            };
            
            daily_stats.insert(date, DailyStats {
                runs: runs.into_iter().cloned().collect(),
                avg_isk,
                avg_time,
                avg_iskph,
            });
        }

        // 전체 통계 생성
        let overall_avg_isk = if runs_data.is_empty() { 0.0 } else {
            runs_data.iter().map(|r| r.net_profit).sum::<f64>() / runs_data.len() as f64
        };
        let overall_avg_time = if runs_data.is_empty() { 0.0 } else {
            runs_data.iter().map(|r| r.run_time_minutes).sum::<f64>() / runs_data.len() as f64
        };
        let overall_avg_iskph = if runs_data.is_empty() { 0.0 } else {
            runs_data.iter().map(|r| r.isk_per_hour).sum::<f64>() / runs_data.len() as f64
        };

        // 티어/웨더별 통계
        let mut tier_weather_groups: HashMap<(String, String), Vec<&RunData>> = HashMap::new();
        for run in &runs_data {
            let parts: Vec<&str> = run.abyssal_type.split_whitespace().collect();
            if parts.len() >= 2 {
                let tier = parts[0].to_string();
                let weather = parts[1].to_string();
                tier_weather_groups.entry((tier, weather)).or_insert_with(Vec::new).push(run);
            }
        }

        let mut tier_weather_stats = Vec::new();
        for ((tier, weather), runs) in tier_weather_groups {
            if !runs.is_empty() {
                let avg_isk = runs.iter().map(|r| r.net_profit).sum::<f64>() / runs.len() as f64;
                let avg_time = runs.iter().map(|r| r.run_time_minutes).sum::<f64>() / runs.len() as f64;
                let avg_iskph = runs.iter().map(|r| r.isk_per_hour).sum::<f64>() / runs.len() as f64;
                
                tier_weather_stats.push(TierWeatherStats {
                    tier,
                    weather,
                    runs_count: runs.len(),
                    avg_isk,
                    avg_time,
                    avg_iskph,
                });
            }
        }

        let overall_stats = OverallStats {
            avg_isk: overall_avg_isk,
            avg_time: overall_avg_time,
            avg_iskph: overall_avg_iskph,
            tier_weather_stats,
        };

        let end_analysis = start_analysis.elapsed();
        self.emit_progress("analysis", &format!("데이터 분석 및 통계 생성 완료 ({:.2}초)", end_analysis.as_secs_f64()), Some(100.0), true);
        println!("  ▶️ 데이터 분석 및 통계 생성 완료. 소요 시간: {:.2}초 ✅", end_analysis.as_secs_f64());

        let end_total = start_total.elapsed();
        self.emit_progress("complete", &format!("전체 데이터 로딩 및 분석 완료! (총 {:.2}초)", end_total.as_secs_f64()), Some(100.0), true);
        println!("✨ [AbyssalDataAnalyzer] 전체 데이터 로딩 및 분석 완료. 총 소요 시간: {:.2}초 ✨", end_total.as_secs_f64());
        
        Ok(AnalysisResult {
            df: runs_data,
            daily_stats,
            overall_stats,
            item_buy_price_cache,
        })
    }

    // 가벼운 데이터 분석 - 기존 캐시된 가격 정보 활용
    pub async fn light_analyze_data(&mut self, df: DataFrame) -> Result<AnalysisResult, anyhow::Error> {
        println!("🚀 [AbyssalDataAnalyzer] 가벼운 데이터 분석 시작 (캐시된 가격 정보 활용)");
        let start_total = std::time::Instant::now();

        if df.height() == 0 {
            println!("⚠️ 분석할 데이터가 없습니다.");
            return Ok(AnalysisResult {
                df: Vec::new(),
                daily_stats: HashMap::new(),
                overall_stats: OverallStats {
                    avg_isk: 0.0,
                    avg_time: 0.0,
                    avg_iskph: 0.0,
                    tier_weather_stats: Vec::new(),
                },
                item_buy_price_cache: HashMap::new(),
            });
        }

        // 새로운 아이템들 수집
        let mut all_item_names = std::collections::HashSet::new();
        let acquired_items_col = df.column("획득 아이템")
            .map_err(|e| anyhow::anyhow!("획득 아이템 컬럼 없음: {}", e))?
            .str().unwrap();
        
        let data_manager = self.data_manager.lock().await;
        for i in 0..df.height() {
            if let Some(items_str) = acquired_items_col.get(i) {
                let parsed_items = data_manager.parse_items(items_str);
                for (name, _) in parsed_items {
                    all_item_names.insert(name);
                }
            }
        }

        // 어비셜 타입에서 필라멘트 이름 추출
        let abyssal_type_col = df.column("어비셜 종류")
            .map_err(|e| anyhow::anyhow!("어비셜 종류 컬럼 없음: {}", e))?
            .str().unwrap();
        
        for i in 0..df.height() {
            if let Some(abyssal_type) = abyssal_type_col.get(i) {
                if let Some(filament_name) = data_manager.abyssal_type_to_filament_name(abyssal_type) {
                    all_item_names.insert(filament_name);
                }
            }
        }
        drop(data_manager);

        println!("  ▶️ 총 {}개의 고유 아이템 발견", all_item_names.len());

        // 기존 캐시에서 가격 정보 로드 시도
        let mut item_buy_price_cache = HashMap::new();
        let mut item_sell_price_cache = HashMap::new();
        let mut missing_items = Vec::new();

        // 모든 아이템을 missing으로 처리 (실제 캐시 로직은 EVEApi에서 처리됨)
        for item_name in &all_item_names {
            missing_items.push(item_name.clone());
        }

        // 새로운 아이템이 있으면 API 호출
        if !missing_items.is_empty() {
            println!("  ▶️ {}개의 새로운 아이템에 대해 API 조회 중...", missing_items.len());
            
            // TypeID 조회
            let name_to_id = self.eve_api.fetch_type_ids(missing_items).await?;
            
            // 가격 조회
            let type_ids: Vec<u32> = name_to_id.values().cloned().collect();
            let prices = self.eve_api.fetch_fuzzwork_prices(type_ids).await?;
            
            // 가격 캐시 업데이트
            for (name, type_id) in &name_to_id {
                if let Some(price_data) = prices.get(&type_id.to_string()) {
                    if let Some(buy_max_str) = price_data.get("buy")
                        .and_then(|buy| buy.get("max"))
                        .and_then(|max| max.as_str()) {
                        if let Ok(buy_max) = buy_max_str.parse::<f64>() {
                            item_buy_price_cache.insert(name.clone(), buy_max);
                        }
                    }
                    
                    if let Some(sell_min_str) = price_data.get("sell")
                        .and_then(|sell| sell.get("min"))
                        .and_then(|min| min.as_str()) {
                        if let Ok(sell_min) = sell_min_str.parse::<f64>() {
                            item_sell_price_cache.insert(name.clone(), sell_min);
                        }
                    }
                }
            }
            println!("  ▶️ 새로운 아이템 가격 조회 완료");
        } else {
            println!("  ▶️ 모든 아이템이 캐시에 있음, API 호출 생략");
        }

        // 런 데이터 계산 (기존과 동일한 로직)
        let mut runs_data = Vec::new();
        
        let start_time_col = df.column("시작시각(KST)").unwrap().str().unwrap();
        let end_time_col = df.column("종료시각(KST)").unwrap().str().unwrap();
        let run_time_col = df.column("런 소요(분)").unwrap().f64().unwrap();
        let abyssal_type_col = df.column("어비셜 종류").unwrap().str().unwrap();
        let acquired_items_col = df.column("획득 아이템").unwrap().str().unwrap();
        
        let data_manager = self.data_manager.lock().await;
        
        for i in 0..df.height() {
            let start_time = start_time_col.get(i).unwrap_or("").to_string();
            let end_time = end_time_col.get(i).unwrap_or("").to_string();
            let run_time_minutes = run_time_col.get(i).unwrap_or(0.0);
            let abyssal_type = abyssal_type_col.get(i).unwrap_or("").to_string();
            let acquired_items = acquired_items_col.get(i).unwrap_or("").to_string();
            
            // 드롭 가격 계산
            let drop_value: f64 = {
                let parsed_items = data_manager.parse_items(&acquired_items);
                parsed_items.into_iter().map(|(name, qty)| {
                    let price = item_buy_price_cache.get(&name).unwrap_or(&0.0);
                    price * (qty as f64)
                }).sum()
            };
            
            // 입장료 계산
            let entry_cost: f64 = {
                if let Some(filament) = data_manager.abyssal_type_to_filament_name(&abyssal_type) {
                    let price = item_sell_price_cache.get(&filament).unwrap_or(&0.0);
                    price * 3.0
                } else {
                    0.0
                }
            };
            
            let net_profit = drop_value - entry_cost;
            let isk_per_hour = if run_time_minutes > 0.0 {
                net_profit / (run_time_minutes / 60.0)
            } else {
                0.0
            };
            
            let date = if start_time.len() >= 10 {
                start_time[0..10].to_string()
            } else {
                "".to_string()
            };
            
            runs_data.push(RunData {
                start_time,
                end_time,
                run_time_minutes,
                abyssal_type,
                net_profit,
                isk_per_hour,
                acquired_items,
                date,
                drop_value,
                entry_cost,
            });
        }
        
        drop(data_manager);

        // 통계 계산 (기존과 동일)
        let mut daily_stats = HashMap::new();
        let mut grouped_by_date: HashMap<String, Vec<&RunData>> = HashMap::new();
        
        for run in &runs_data {
            grouped_by_date.entry(run.date.clone()).or_insert_with(Vec::new).push(run);
        }

        for (date, runs) in grouped_by_date {
            let avg_isk = if runs.is_empty() { 0.0 } else { 
                runs.iter().map(|r| r.net_profit).sum::<f64>() / runs.len() as f64 
            };
            let avg_time = if runs.is_empty() { 0.0 } else { 
                runs.iter().map(|r| r.run_time_minutes).sum::<f64>() / runs.len() as f64 
            };
            let avg_iskph = if runs.is_empty() { 0.0 } else { 
                runs.iter().map(|r| r.isk_per_hour).sum::<f64>() / runs.len() as f64 
            };
            
            daily_stats.insert(date, DailyStats {
                runs: runs.into_iter().cloned().collect(),
                avg_isk,
                avg_time,
                avg_iskph,
            });
        }

        // 전체 통계 생성
        let overall_avg_isk = if runs_data.is_empty() { 0.0 } else {
            runs_data.iter().map(|r| r.net_profit).sum::<f64>() / runs_data.len() as f64
        };
        let overall_avg_time = if runs_data.is_empty() { 0.0 } else {
            runs_data.iter().map(|r| r.run_time_minutes).sum::<f64>() / runs_data.len() as f64
        };
        let overall_avg_iskph = if runs_data.is_empty() { 0.0 } else {
            runs_data.iter().map(|r| r.isk_per_hour).sum::<f64>() / runs_data.len() as f64
        };

        // 티어/웨더별 통계
        let mut tier_weather_groups: HashMap<(String, String), Vec<&RunData>> = HashMap::new();
        for run in &runs_data {
            let parts: Vec<&str> = run.abyssal_type.split_whitespace().collect();
            if parts.len() >= 2 {
                let tier = parts[0].to_string();
                let weather = parts[1].to_string();
                tier_weather_groups.entry((tier, weather)).or_insert_with(Vec::new).push(run);
            }
        }

        let mut tier_weather_stats = Vec::new();
        for ((tier, weather), runs) in tier_weather_groups {
            if !runs.is_empty() {
                let avg_isk = runs.iter().map(|r| r.net_profit).sum::<f64>() / runs.len() as f64;
                let avg_time = runs.iter().map(|r| r.run_time_minutes).sum::<f64>() / runs.len() as f64;
                let avg_iskph = runs.iter().map(|r| r.isk_per_hour).sum::<f64>() / runs.len() as f64;
                
                tier_weather_stats.push(TierWeatherStats {
                    tier,
                    weather,
                    runs_count: runs.len(),
                    avg_isk,
                    avg_time,
                    avg_iskph,
                });
            }
        }

        let overall_stats = OverallStats {
            avg_isk: overall_avg_isk,
            avg_time: overall_avg_time,
            avg_iskph: overall_avg_iskph,
            tier_weather_stats,
        };

        let end_total = start_total.elapsed();
        println!("✨ [AbyssalDataAnalyzer] 가벼운 데이터 분석 완료. 소요 시간: {:.2}초 ✨", end_total.as_secs_f64());
        
        Ok(AnalysisResult {
            df: runs_data,
            daily_stats,
            overall_stats,
            item_buy_price_cache,
        })
    }
}