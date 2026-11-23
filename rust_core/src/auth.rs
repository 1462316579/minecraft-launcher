//! 认证模块：Microsoft / Mojang 账号登录

use serde::Deserialize;

/// 登录响应
#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub user_name: String,
    pub uuid: String,
}

/// 使用 Microsoft 代码登录（简化：直接通过用户名/密码或已有 token）
/// 实际生产中应使用 OAuth2 设备码流程
pub async fn microsoft_login(
    email: &str,
    password: &str,
) -> Result<AuthResponse, String> {
    // 注意：Microsoft 已不再支持直接用户名密码登录，
    // 这里仅作为框架占位，实际需接 OAuth2 设备码或 WebView 登录
    Err("Microsoft 登录需使用 OAuth2 设备码流程，请通过 WebView 完成".to_string())
}

/// 使用已有 access_token 验证
pub async fn validate_token(access_token: &str) -> Result<AuthResponse, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("验证 token 失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("token 无效: HTTP {}", resp.status()));
    }
    #[derive(Deserialize)]
    struct Profile {
        id: String,
        name: String,
    }
    let profile: Profile = resp.json().await.map_err(|e| format!("解析失败: {}", e))?;
    Ok(AuthResponse {
        access_token: access_token.to_string(),
        user_name: profile.name,
        uuid: profile.id,
    })
}
