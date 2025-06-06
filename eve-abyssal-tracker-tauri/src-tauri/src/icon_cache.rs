use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::Value;
use tokio::fs;
use anyhow::Result;

pub struct IconCache {
    data_dir: PathBuf,
    type_id_cache: HashMap<String, u32>,
}

impl IconCache {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            type_id_cache: HashMap::new(),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // typeid_cache.json 로드
        self.load_type_id_cache().await?;
        Ok(())
    }

    async fn load_type_id_cache(&mut self) -> Result<()> {
        let cache_file = self.data_dir.join("typeid_cache.json");
        println!("[DEBUG] Looking for typeid_cache.json at: {:?}", cache_file);
        
        if cache_file.exists() {
            println!("[DEBUG] typeid_cache.json exists, reading content...");
            let content = fs::read_to_string(&cache_file).await?;
            let json: Value = serde_json::from_str(&content)?;
            
            if let Some(obj) = json.as_object() {
                for (name, id) in obj {
                    if let Some(id_num) = id.as_u64() {
                        self.type_id_cache.insert(name.clone(), id_num as u32);
                    }
                }
            }
            
            println!("[INFO] Loaded {} type IDs from cache", self.type_id_cache.len());
            println!("[DEBUG] Sample items: {:?}", self.type_id_cache.keys().take(5).collect::<Vec<_>>());
        } else {
            println!("[ERROR] typeid_cache.json not found at: {:?}", cache_file);
        }
        
        Ok(())
    }

    pub fn get_type_id(&self, item_name: &str) -> Option<u32> {
        // 아이템 이름에서 * 제거 후 조회
        let cleaned_name = item_name.replace('*', "").trim().to_string();
        
        // 원본 이름으로 먼저 시도
        if let Some(type_id) = self.type_id_cache.get(item_name) {
            return Some(*type_id);
        }
        
        // * 제거한 이름으로 시도
        if let Some(type_id) = self.type_id_cache.get(&cleaned_name) {
            return Some(*type_id);
        }
        
        println!("[DEBUG] No type_id found for item: '{}' (cleaned: '{}')", item_name, cleaned_name);
        println!("[DEBUG] Available items in cache: {:?}", self.type_id_cache.keys().take(5).collect::<Vec<_>>());
        None
    }

    pub async fn get_available_image_types(&self, type_id: u32) -> Result<Vec<String>> {
        let url = format!("https://images.evetech.net/types/{}", type_id);
        
        let response = reqwest::get(&url).await?;
        if response.status().is_success() {
            let types: Vec<String> = response.json().await?;
            println!("[DEBUG] Available types for {}: {:?}", type_id, types);
            Ok(types)
        } else {
            println!("[DEBUG] Failed to get image types for {}: {}", type_id, response.status());
            Ok(vec!["icon".to_string()]) // 기본값으로 icon 반환
        }
    }

    pub async fn get_best_image_url(&self, type_id: u32, item_name: &str) -> Result<String> {
        // 아이템 이름에 Blueprint가 포함되어 있으면 bp 사용
        let image_type = if item_name.contains("Blueprint") {
            "bp"
        } else {
            "icon"
        };
        
        let url = format!("https://images.evetech.net/types/{}/{}", type_id, image_type);
        Ok(url)
    }

    pub fn get_icon_url(&self, type_id: u32) -> String {
        format!("https://images.evetech.net/types/{}/icon", type_id)
    }

    pub fn get_multiple_type_ids(&self, item_names: &[String]) -> HashMap<String, u32> {
        let mut result = HashMap::new();
        for name in item_names {
            if let Some(type_id) = self.get_type_id(name) {
                result.insert(name.clone(), type_id);
            }
        }
        result
    }
} 