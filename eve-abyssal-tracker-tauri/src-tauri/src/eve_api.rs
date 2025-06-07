use std::{collections::HashMap, path::PathBuf, sync::Arc, time::{SystemTime, UNIX_EPOCH}};
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
const PRICE_CACHE_FILE: &str = "data/price_cache.json"; // 가격 캐시 파일
const PRICE_CACHE_TTL_SECONDS: u64 = 30 * 60; // 30분 TTL

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPriceData {
    pub prices: HashMap<String, serde_json::Value>,
    pub cached_at: u64, // Unix timestamp
}

impl CachedPriceData {
    fn new(prices: HashMap<String, serde_json::Value>) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            prices,
            cached_at,
        }
    }
    
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now - self.cached_at > PRICE_CACHE_TTL_SECONDS
    }
}

pub struct EVEApi {
    client: Client,
    name_to_id_cache: Arc<Mutex<HashMap<String, u32>>>, // Python과 동일한 구조
    price_cache: Arc<Mutex<Option<CachedPriceData>>>, // 가격 캐시
}

impl EVEApi {
    pub async fn new(_app_handle: &AppHandle) -> Result<Self> {
        let name_to_id_cache = Arc::new(Mutex::new(HashMap::new()));
        let price_cache = Arc::new(Mutex::new(None));

        let api = Self {
            client: Client::new(),
            name_to_id_cache,
            price_cache,
        };

        api.load_cache().await?;
        api.load_price_cache().await?;
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

    async fn load_price_cache(&self) -> Result<()> {
        let cache_path = PathBuf::from(PRICE_CACHE_FILE);
        if cache_path.exists() {
            let content = fs::read_to_string(&cache_path).await?;
            let loaded_cache: CachedPriceData = serde_json::from_str(&content)?;
            
            if !loaded_cache.is_expired() {
                let cached_at = loaded_cache.cached_at;
                let prices_count = loaded_cache.prices.len();
                *self.price_cache.lock().await = Some(loaded_cache);
                
                let remaining_minutes = (PRICE_CACHE_TTL_SECONDS - (SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() - cached_at)) / 60;
                
                println!("[INFO] 가격 캐시 로드 성공 ({}개 아이템, 남은 시간: {}분)", 
                    prices_count, remaining_minutes);
            } else {
                println!("[INFO] 가격 캐시가 만료되어 삭제합니다.");
                let _ = fs::remove_file(&cache_path).await;
            }
        }
        Ok(())
    }

    async fn save_price_cache(&self, prices: &HashMap<String, serde_json::Value>) -> Result<()> {
        let cache_path = PathBuf::from(PRICE_CACHE_FILE);
        
        // data 디렉토리 생성
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let cached_data = CachedPriceData::new(prices.clone());
        let content = serde_json::to_string_pretty(&cached_data)?;
        fs::write(&cache_path, content).await?;
        
        *self.price_cache.lock().await = Some(cached_data);
        println!("[INFO] 가격 캐시 저장 완료 ({}개 아이템, 30분 TTL)", prices.len());
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

        let cached_count = name_to_id.len();
        if cached_count > 0 {
            println!("[INFO] type_id 캐시에서 {}개 아이템 조회", cached_count);
        }

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

        // 캐시 확인
        let cache_guard = self.price_cache.lock().await;
        if let Some(cached_data) = cache_guard.as_ref() {
            if !cached_data.is_expired() {
                // 요청된 모든 ID가 캐시에 있는지 확인
                let ids_str_set: std::collections::HashSet<String> = ids.iter().map(|id| id.to_string()).collect();
                let cached_ids_set: std::collections::HashSet<String> = cached_data.prices.keys().cloned().collect();
                
                if ids_str_set.is_subset(&cached_ids_set) {
                    // 캐시에서 요청된 항목들만 추출
                    let mut result = HashMap::new();
                    for id in &ids {
                        let id_str = id.to_string();
                        if let Some(price_data) = cached_data.prices.get(&id_str) {
                            result.insert(id_str, price_data.clone());
                        }
                    }
                    let remaining_minutes = (PRICE_CACHE_TTL_SECONDS - (SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() - cached_data.cached_at)) / 60;
                    
                    println!("[INFO] 가격 캐시에서 {}개 아이템 조회 (남은 시간: {}분)", result.len(), remaining_minutes);
                    drop(cache_guard);
                    return Ok(result);
                }
            }
        }
        drop(cache_guard);

        // 캐시 미스 또는 만료된 경우 API 호출
        println!("[INFO] 가격 캐시 미스, Fuzzwork API 호출 중...");
        
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
                            
                            // 새로운 데이터를 캐시에 저장
                            if let Err(e) = self.save_price_cache(&data).await {
                                println!("[WARNING] 가격 캐시 저장 실패: {}", e);
                            }
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