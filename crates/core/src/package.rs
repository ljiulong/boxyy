use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub manager: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub installed_path: Option<String>,
    pub size: Option<u64>,
    pub outdated: bool,
    pub latest_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatus {
    pub name: String,
    pub version: String,
    pub available: bool,
    pub package_count: usize,
    pub outdated_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub manager: String,
    pub operation: Operation,
    pub target: String,
    pub status: JobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub logs: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Install,
    Update,
    Uninstall,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Capability {
    ListInstalled,
    SearchRemote,
    QueryDependencies,
    VersionSelection,
    BatchInstall,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_serialization() {
        let package = Package {
            name: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            manager: "npm".to_string(),
            description: Some("Test package".to_string()),
            homepage: Some("https://example.com".to_string()),
            license: Some("MIT".to_string()),
            installed_path: Some("/usr/local/lib/node_modules".to_string()),
            size: Some(1024),
            outdated: false,
            latest_version: Some("1.0.0".to_string()),
        };

        let json = serde_json::to_string(&package).unwrap();
        let deserialized: Package = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, package.name);
        assert_eq!(deserialized.version, package.version);
        assert_eq!(deserialized.manager, package.manager);
    }
}
