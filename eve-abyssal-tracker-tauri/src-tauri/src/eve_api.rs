use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use anyhow::{Result, anyhow};
use tokio::fs;
use tauri::{AppHandle, Manager};

// Python과 동일한 상수값들
const REGION_ID: u32 = 10000002; // The Forge
const STATION_ID: u32 = 60003760; // Jita 4-4
const CACHE_FILE: &str = "data/typeid_cache.json"; // Python과 동일한 경로

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeIdResponse {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzworkPrice {
    pub buy: FuzzworkPriceDetail,
    pub sell: FuzzworkPriceDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzworkPriceDetail {
    pub volume: u64,
    pub weighted_average: f64,
    pub max: f64,
    pub min: f64,
    pub stddev: f64,
    pub median: f64,
    pub order_count: u64,
    pub percentiles: HashMap<String, f64>,
}

pub struct EVEApi {
    client: Client,
    name_to_id_cache: Arc<Mutex<HashMap<String, u32>>>, // Python과 동일한 구조
}

impl EVEApi {
    pub async fn new(_app_handle: &AppHandle) -> Result<Self> {
        let name_to_id_cache = Arc::new(Mutex::new(HashMap::new()));

        let api = Self {
            client: Client::new(),
            name_to_id_cache,
        };

        api.load_cache().await?;
        Ok(api)
    }

    async fn load_cache(&self) -> Result<()> {
        let cache_path = PathBuf::from(CACHE_FILE);
        if cache_path.exists() {
            let content = fs::read_to_string(&cache_path).await?;
            let loaded_cache: HashMap<String, u32> = serde_json::from_str(&content)?;
            *self.name_to_id_cache.lock().await = loaded_cache;
        }
        Ok(())
    }

    async fn save_cache(&self) -> Result<()> {
        let cache_path = PathBuf::from(CACHE_FILE);
        
        // data 디렉토리 생성
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let content = serde_json::to_string_pretty(&*self.name_to_id_cache.lock().await)?;
        fs::write(&cache_path, content).await?;
        Ok(())
    }

    pub async fn fetch_type_ids(&self, names: Vec<String>) -> Result<HashMap<String, u32>> {
        let mut name_to_id = HashMap::new();
        let mut names_to_query = Vec::new();

        let cache = self.name_to_id_cache.lock().await;
        
        // 캐시에서 확인
        for name in &names {
            if let Some(&id) = cache.get(name) {
                name_to_id.insert(name.clone(), id);
            } else {
                names_to_query.push(name.clone());
            }
        }
        drop(cache); // 락 해제

        if names_to_query.is_empty() {
            println!("[INFO] 모든 아이템 type_id가 캐시에 존재합니다. API 호출 건너뜀.");
            return Ok(name_to_id);
        }

        println!("[INFO] ESI API로 {}개의 아이템 type_id 조회 시작...", names_to_query.len());
        
        // Python과 동일하게 20개씩 청크로 나누어 처리
        for (i, chunk) in names_to_query.chunks(20).enumerate() {
            println!("[INFO] ESI API 호출 중... ({}/{})", i * 20 + 1, names_to_query.len());
            
            match self.fetch_chunk_type_ids(chunk.to_vec()).await {
                Ok(chunk_results) => {
                    for (name, id) in chunk_results {
                        name_to_id.insert(name.clone(), id);
                        // 캐시에 추가
                        self.name_to_id_cache.lock().await.insert(name, id);
                    }
                }
                Err(e) => {
                    println!("[ERROR] ESI API 호출 중 오류 발생: {}, 청크: {:?}", e, chunk);
                }
            }
            
            // Python과 동일한 지연
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        self.save_cache().await?;
        println!("[INFO] ESI API type_id 조회 완료. 총 {}개 변환 성공.", name_to_id.len());
        Ok(name_to_id)
    }

    async fn fetch_chunk_type_ids(&self, chunk: Vec<String>) -> Result<HashMap<String, u32>> {
        let mut result = HashMap::new();
        
        let url = "https://esi.evetech.net/latest/universe/ids/";
        let response = self.client
            .post(url)
            .header("User-Agent", "bulk-lookup 1.0")
            .header("Content-Type", "application/json")
            .json(&chunk)
            .timeout(tokio::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("ESI API HTTP 오류: {}", response.status()));
        }

        let response_data: serde_json::Value = response.json().await?;
        
        if response_data.is_null() {
            println!("[WARNING] ESI API 응답이 비어있습니다. 청크: {:?}", chunk);
            return Ok(result);
        }

        if let Some(inventory_types) = response_data.get("inventory_types").and_then(|v| v.as_array()) {
            for item in inventory_types {
                if let (Some(name), Some(id)) = (
                    item.get("name").and_then(|v| v.as_str()),
                    item.get("id").and_then(|v| v.as_u64())
                ) {
                    result.insert(name.to_string(), id as u32);
                }
            }
        }

        Ok(result)
    }

    pub async fn fetch_fuzzwork_prices(&self, ids: Vec<u32>) -> Result<HashMap<String, serde_json::Value>> {
        if ids.is_empty() {
            println!("[INFO] Fuzzwork API에 조회할 type_id가 없습니다.");
            return Ok(HashMap::new());
        }

        let ids_str: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        let url = format!(
            "https://market.fuzzwork.co.uk/aggregates/?region={}&types={}",
            REGION_ID,
            ids_str.join(",")
        );

        match self.client
            .get(&url)
            .timeout(tokio::time::Duration::from_secs(15))
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    println!("[ERROR] Fuzzwork API HTTP 오류: {} - URL: {}", response.status(), url);
                    return Ok(HashMap::new());
                }

                match response.json::<HashMap<String, serde_json::Value>>().await {
                    Ok(data) => {
                        if data.is_empty() {
                            println!("[WARNING] Fuzzwork API 응답이 비어있습니다. URL: {}", url);
                        } else {
                            println!("[INFO] Fuzzwork API 시세 조회 성공. {}개 아이템.", data.len());
                        }
                        Ok(data)
                    }
                    Err(e) => {
                        println!("[ERROR] Fuzzwork API 응답 JSON 디코딩 오류: {}", e);
                        Ok(HashMap::new())
                    }
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    println!("[ERROR] Fuzzwork API 호출 타임아웃 발생: {}", url);
                } else if e.is_connect() {
                    println!("[ERROR] Fuzzwork API 연결 오류: {}, URL: {}", e, url);
                } else {
                    println!("[ERROR] Fuzzwork API 호출 중 오류 발생: {}", e);
                }
                Ok(HashMap::new())
            }
        }
    }
}

#[tauri::command]
pub async fn get_type_ids(app_handle: AppHandle, item_names: Vec<String>) -> Result<HashMap<String, u32>, String> {
    let eve_api = app_handle.state::<Arc<EVEApi>>();
    eve_api.fetch_type_ids(item_names).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_fuzzwork_prices(app_handle: AppHandle, type_ids: Vec<u32>) -> Result<HashMap<String, serde_json::Value>, String> {
    let eve_api = app_handle.state::<Arc<EVEApi>>();
    eve_api.fetch_fuzzwork_prices(type_ids).await.map_err(|e| e.to_string())
}