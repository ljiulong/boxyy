use boxy_error::{BoxyError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, info};

const CACHE_DIR: &str = "boxy";
const CACHE_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub cache_dir: Option<PathBuf>,
    pub ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: None,
            ttl: CACHE_TTL,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub cached_at: i64,
}

pub struct Cache {
    cache_dir: PathBuf,
    ttl: Duration,
}

impl Cache {
    pub fn new() -> Result<Self> {
        Self::new_with_config(CacheConfig::default())
    }

    pub fn new_with_config(config: CacheConfig) -> Result<Self> {
        let cache_dir = match config.cache_dir {
            Some(dir) => dir,
            None => dirs::cache_dir()
                .ok_or_else(|| BoxyError::CacheError {
                    message: "无法获取缓存目录".to_string(),
                })?
                .join(CACHE_DIR),
        };

        Ok(Self {
            cache_dir,
            ttl: config.ttl,
        })
    }

    async fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| BoxyError::CacheError {
                message: format!("创建缓存目录失败: {}", e),
            })
    }

    pub fn manager_path(&self, manager: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", manager))
    }

    pub async fn get<T>(&self, manager: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let path = self.manager_path(manager);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| BoxyError::CacheError {
                message: format!("读取缓存失败: {}", e),
            })?;

        let entry: CacheEntry<T> =
            serde_json::from_str(&content).map_err(|e| BoxyError::JsonError {
                message: format!("解析缓存失败: {}", e),
            })?;

        let now = chrono::Utc::now().timestamp();
        if now - entry.cached_at > self.ttl.as_secs() as i64 {
            debug!("缓存过期: {}", manager);
            return Ok(None);
        }

        debug!("缓存命中: {}", manager);
        Ok(Some(entry.data))
    }

    pub async fn set<T>(&self, manager: &str, data: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.ensure_dir().await?;

        let path = self.manager_path(manager);
        let entry = CacheEntry {
            data,
            cached_at: chrono::Utc::now().timestamp(),
        };

        let content = serde_json::to_string_pretty(&entry).map_err(|e| BoxyError::JsonError {
            message: format!("序列化缓存失败: {}", e),
        })?;

        fs::write(&path, content)
            .await
            .map_err(|e| BoxyError::CacheError {
                message: format!("写入缓存失败: {}", e),
            })?;

        debug!("缓存已更新: {}", manager);
        Ok(())
    }

    pub async fn invalidate(&self, manager: &str) -> Result<()> {
        let path = self.manager_path(manager);

        if path.exists() {
            fs::remove_file(&path)
                .await
                .map_err(|e| BoxyError::CacheError {
                    message: format!("删除缓存失败: {}", e),
                })?;
            info!("缓存已清除: {}", manager);
        }

        Ok(())
    }

    pub async fn clean(&self, older_than: Duration) -> Result<usize> {
        let mut cleaned = 0;

        let mut entries =
            fs::read_dir(&self.cache_dir)
                .await
                .map_err(|e| BoxyError::CacheError {
                    message: format!("读取缓存目录失败: {}", e),
                })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| BoxyError::CacheError {
                message: format!("读取缓存条目失败: {}", e),
            })?
        {
            let path = entry.path();
            if !matches!(path.extension(), Some(e) if e == "json") {
                continue;
            }

            let metadata = entry.metadata().await.map_err(|e| BoxyError::CacheError {
                message: format!("获取文件元数据失败: {}", e),
            })?;

            let modified = metadata.modified().map_err(|e| BoxyError::CacheError {
                message: format!("获取修改时间失败: {}", e),
            })?;

            let elapsed = std::time::SystemTime::now()
                .duration_since(modified)
                .unwrap_or(Duration::ZERO);

            if elapsed > older_than {
                fs::remove_file(&path)
                    .await
                    .map_err(|e| BoxyError::CacheError {
                        message: format!("删除过期缓存失败: {}", e),
                    })?;
                cleaned += 1;
            }
        }

        if cleaned > 0 {
            info!("已清理 {} 个过期缓存", cleaned);
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cache_set_get() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let mut cache = Cache::new().unwrap();
        cache.cache_dir = cache_dir;

        let data = vec!["package1", "package2"];

        cache.set("npm", &data).await.unwrap();
        let result: Option<Vec<String>> = cache.get("npm").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), data);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let mut cache = Cache::new().unwrap();
        cache.cache_dir = cache_dir;

        let result: Option<Vec<String>> = cache.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        let mut cache = Cache::new().unwrap();
        cache.cache_dir = cache_dir;

        cache.set("npm", &vec!["package1"]).await.unwrap();
        cache.invalidate("npm").await.unwrap();

        let result: Option<Vec<String>> = cache.get("npm").await.unwrap();
        assert!(result.is_none());
    }
}
