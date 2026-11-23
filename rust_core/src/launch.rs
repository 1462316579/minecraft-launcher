//! 启动模块：构建 Minecraft 启动命令并启动 Java 进程

use std::path::Path;

/// 启动参数配置
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub version_id: String,
    pub version_name: String,
    pub game_dir: String,
    pub assets_dir: String,
    pub asset_index_id: String,
    pub auth_uuid: String,
    pub auth_access_token: String,
    pub auth_user_type: String,
    pub main_class: String,
    pub minecraft_args: String,
    pub jvm_args: Vec<String>,
    pub classpath: Vec<String>,
    pub width: i32,
    pub height: i32,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            version_id: "1.20.1".to_string(),
            version_name: "1.20.1".to_string(),
            game_dir: std::env::temp_dir().to_string_lossy().to_string(),
            assets_dir: std::env::temp_dir().to_string_lossy().to_string(),
            asset_index_id: "1.20.1".to_string(),
            auth_uuid: "".to_string(),
            auth_access_token: "".to_string(),
            auth_user_type: "mojang".to_string(),
            main_class: "net.minecraft.client.main.Main".to_string(),
            minecraft_args: "".to_string(),
            jvm_args: vec!["-Xmx2G".to_string(), "-Xms256M".to_string()],
            classpath: vec![],
            width: 854,
            height: 480,
        }
    }
}

/// 构建启动命令行（返回可执行路径和参数列表）
pub fn build_launch_command(config: &LaunchConfig) -> (String, Vec<String>) {
    let java_path = which_java();
    let mut args = Vec::new();

    // JVM 参数
    for jvm_arg in &config.jvm_args {
        args.push(jvm_arg.clone());
    }

    // 类路径
    args.push(format!("-Djava.library.path={}/natives", config.game_dir));
    args.push("-cp".to_string());
    args.push(config.classpath.join(std::path::MAIN_SEPARATOR_STR));

    // 主类
    args.push(config.main_class.clone());

    // Minecraft 启动参数
    args.push(config.minecraft_args.clone());

    // 替换占位符（简化）
    let args: Vec<String> = args
        .iter()
        .map(|a| {
            a.replace("${auth_player_name}", &config.auth_user_type)
                .replace("${version_name}", &config.version_name)
                .replace("${game_directory}", &config.game_dir)
                .replace("${assets_root}", &config.assets_dir)
                .replace("${asset_index_name}", &config.asset_index_id)
                .replace("${auth_access_token}", &config.auth_access_token)
                .replace("${auth_uuid}", &config.auth_uuid)
                .replace("${auth_user_type}", &config.auth_user_type)
                .replace("${version_type}", "release")
                .replace("${user_type}", "mojang")
                .replace("${resolution_width}", &config.width.to_string())
                .replace("${resolution_height}", &config.height.to_string())
                .replace("${natives_directory}", &format!("{}/natives", config.game_dir))
                .replace("${launcher_name}", "Minecraft Launcher")
                .replace("${launcher_version}", "1.0.0")
                .replace("${classpath}", &config.classpath.join(std::path::MAIN_SEPARATOR_STR))
        })
        .collect();

    (java_path, args)
}

/// 查找 Java 可执行文件路径
fn which_java() -> String {
    // 优先使用 JAVA_HOME/bin/java
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let path = format!("{}/bin/java", java_home);
        if std::path::Path::new(&path).exists() {
            return path;
        }
    }
    "java".to_string()
}

/// 启动 Minecraft 进程（异步，返回子进程句柄）
pub async fn launch_minecraft(config: LaunchConfig) -> Result<tokio::process::Child, String> {
    let (java_path, args) = build_launch_command(&config);

    let mut cmd = tokio::process::Command::new(java_path);
    cmd.args(&args)
        .current_dir(&config.game_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 Java 进程失败: {}", e))?;

    Ok(child)
}

/// 生成默认 classpath（需预先下载所有库文件和客户端 jar）
pub fn build_classpath(game_dir: &str, client_jar: &str, libs: &[String]) -> Vec<String> {
    let mut cp = Vec::new();
    cp.push(format!("{}/{}", game_dir, client_jar));
    for lib in libs {
        cp.push(lib.clone());
    }
    cp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command() {
        let config = LaunchConfig::default();
        let (java, args) = build_launch_command(&config);
        assert!(!java.is_empty());
        assert!(!args.is_empty());
        assert!(args.iter().any(|a| a.contains("net.minecraft.client.main.Main")));
    }
}
