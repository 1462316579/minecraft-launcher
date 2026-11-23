//! 版本管理：解析 Minecraft 版本 JSON、获取版本列表

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minecraft 版本清单条目
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub latest: VersionLatest,
    pub versions: Vec<VersionEntry>,
}

/// 最新版本引用
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionLatest {
    pub release: String,
    pub snapshot: String,
}

/// 单个版本条目
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionEntry {
    pub id: String,
    #[serde(default)]
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
}

/// 完整版本 JSON（版本元数据）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionJson {
    pub id: String,
    pub inherits_from: Option<String>,
    pub arguments: Option<serde_json::Value>,
    pub downloads: VersionDownloads,
    pub libraries: Vec<Library>,
    pub asset_index: AssetIndex,
    pub main_class: String,
    pub minecraft_arguments: Option<String>,
}

/// 下载资源
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionDownloads {
    pub client: DownloadInfo,
    pub server: Option<DownloadInfo>,
}

/// 下载文件信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: i64,
    pub url: String,
}

/// 库文件
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<LibraryRule>,
}

/// 库下载信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadInfo>,
}

/// 库规则
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryRule {
    pub action: String,
    #[serde(default)]
    pub os: Option<LibraryOs>,
}

/// 操作系统规则
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryOs {
    pub name: String,
}

/// 资源索引
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: i64,
    pub total_size: i64,
    pub url: String,
}

/// 公告数据模型（对应 FCL Announcement.java）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Announcement {
    pub title: String,
    pub content: String,
    pub date: String,
    pub significant: bool,
    #[serde(default)]
    pub should_display: bool,
}

/// 公告下载源（双语分流，对应 FCL ANNOUNCEMENTURL / ANNOUNCEMENTURLCN）
pub const ANNOUNCEMENT_URL: &str = "https://raw.githubusercontent.com/.../announcementv2.txt";
pub const ANNOUNCEMENT_URL_CN: &str = "https://gitee.com/.../announcementv2.txt";

/// 拉取公告 JSON
pub async fn fetch_announcement(cn: bool) -> Result<Announcement, String> {
    let url = if cn { ANNOUNCEMENT_URL_CN } else { ANNOUNCEMENT_URL };
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("获取公告失败: {}", e))?;
    let announcement: Announcement = resp
        .json()
        .await
        .map_err(|e| format!("解析公告失败: {}", e))?;
    Ok(announcement)
}

/// 版本清单下载地址（官方）
pub const VERSION_MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

/// 拉取版本清单
pub async fn fetch_version_manifest() -> Result<VersionManifest, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(VERSION_MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("获取版本清单失败: {}", e))?;
    let manifest: VersionManifest = resp
        .json()
        .await
        .map_err(|e| format!("解析版本清单失败: {}", e))?;
    Ok(manifest)
}

/// 获取指定版本的详细 JSON
pub async fn fetch_version_json(url: &str) -> Result<VersionJson, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("获取版本 JSON 失败: {}", e))?;
    let version: VersionJson = resp
        .json()
        .await
        .map_err(|e| format!("解析版本 JSON 失败: {}", e))?;
    Ok(version)
}

/// 过滤出适用于当前平台的库文件
pub fn filter_applicable_libraries(libs: &[Library]) -> Vec<&Library> {
    libs.iter()
        .filter(|lib| {
            let apply = lib.rules.iter().all(|rule| {
                if let Some(os) = &rule.os {
                    if os.name == "windows" || os.name == "osx" || os.name == "linux" {
                        return false; // 非当前平台规则（简化处理）
                    }
                }
                true
            });
            apply
        })
        .collect()
}

/// 获取库文件下载 URL（Maven 路径）
pub fn library_download_url(lib: &Library) -> Option<String> {
    let parts: Vec<&str> = lib.name.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let base = "https://libraries.minecraft.net/";
    Some(format!("{}{}/{}/{}/{}-{}.jar", group, artifact, version, base, artifact, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_url() {
        let lib = Library {
            name: "com.mojang:patchy:1.1".to_string(),
            downloads: None,
            rules: vec![],
        };
        let url = library_download_url(&lib);
        assert!(url.is_some());
        assert!(url.unwrap().contains("patchy"));
    }
}
