use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::Value;
use tokio::fs;
use anyhow::Result;
use log::*;

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

    pub async fn reload(&mut self) -> Result<()> {
        // 캐시 클리어 후 다시 로드
        self.type_id_cache.clear();
        self.load_type_id_cache().await?;
        info!("IconCache reloaded with {} type IDs", self.type_id_cache.len());
        Ok(())
    }

    async fn load_type_id_cache(&mut self) -> Result<()> {
        let cache_file = self.data_dir.join("typeid_cache.json");
        
        if cache_file.exists() {
            let content = fs::read_to_string(&cache_file).await?;
            let json: Value = serde_json::from_str(&content)?;
            
            if let Some(obj) = json.as_object() {
                for (name, id) in obj {
                    if let Some(id_num) = id.as_u64() {
                        self.type_id_cache.insert(name.clone(), id_num as u32);
                    }
                }
            }
            
            info!("IconCache loaded {} type IDs from cache", self.type_id_cache.len());
        } else {
            warn!("typeid_cache.json not found at: {:?}, will be created when needed", cache_file);
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
        
        // 정규화된 매칭 시도 (공백 정리, 대소문자 무시)
        let normalized_search = cleaned_name.to_lowercase().replace(&[' ', '\t', '\n', '\r'][..], " ");
        let normalized_search = normalized_search.split_whitespace().collect::<Vec<_>>().join(" ");
        
        for (cached_name, type_id) in &self.type_id_cache {
            let normalized_cached = cached_name.to_lowercase().replace(&[' ', '\t', '\n', '\r'][..], " ");
            let normalized_cached = normalized_cached.split_whitespace().collect::<Vec<_>>().join(" ");
            
            if normalized_cached == normalized_search {
                return Some(*type_id);
            }
        }
        
        // 부분 매칭 시도 (검색어가 캐시된 이름에 포함되는 경우)
        for (cached_name, type_id) in &self.type_id_cache {
            let normalized_cached = cached_name.to_lowercase().replace(&[' ', '\t', '\n', '\r'][..], " ");
            let normalized_cached = normalized_cached.split_whitespace().collect::<Vec<_>>().join(" ");
            
            if normalized_cached.contains(&normalized_search) || normalized_search.contains(&normalized_cached) {
                debug!("IconCache: Partial match found for '{}' -> '{}' (type_id: {})", 
                       item_name, cached_name, type_id);
                return Some(*type_id);
            }
        }
        
        // 매칭 실패 시 디버그 정보 출력
        warn!("IconCache: Failed to find type_id for item '{}' (cleaned: '{}', normalized: '{}')", 
              item_name, cleaned_name, normalized_search);
        
        None
    }

    pub async fn get_available_image_types(&self, type_id: u32) -> Result<Vec<String>> {
        let url = format!("https://images.evetech.net/types/{}", type_id);
        
        let response = reqwest::get(&url).await?;
        if response.status().is_success() {
            let types: Vec<String> = response.json().await?;
            Ok(types)
        } else {
            Ok(vec!["icon".to_string()]) // 기본값으로 icon 반환
        }
    }

    pub async fn get_best_image_url(&self, type_id: u32, item_name: &str) -> Result<String> {
        // 아이템 이름에 Blueprint가 있으면 bp, 아니면 icon
        if item_name.contains("Blueprint") {
            let bp_url = format!("https://images.evetech.net/types/{}/bp", type_id);
            Ok(bp_url)
        } else {
            let icon_url = format!("https://images.evetech.net/types/{}/icon", type_id);
            Ok(icon_url)
        }
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