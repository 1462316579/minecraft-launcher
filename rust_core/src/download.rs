//! 下载模块：下载游戏客户端、库文件、资源索引

use futures_util::StreamExt;
use sha1::Digest;
use std::path::Path;

/// 下载单个文件到指定路径，返回字节数
pub async fn download_file(url: &str, dest: &Path) -> Result<u64, String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {}", e))?;

    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("创建文件失败: {}", e))?;

    // reqwest 0.12 Response 实现 AsyncRead（启用 stream feature 后）
    let mut stream = resp.bytes_stream();
    let mut written: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("读取流失败: {}", e))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("写入文件失败: {}", e))?;
        written += chunk.len() as u64;
    }

    Ok(written)
}

/// 下载带进度回调的文件
pub async fn download_file_with_progress<F>(
    url: &str,
    dest: &Path,
    mut on_progress: F,
) -> Result<u64, String>
where
    F: FnMut(u64, u64) + Send,
{
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {}", e))?;

    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("创建文件失败: {}", e))?;
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("读取流失败: {}", e))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("写入文件失败: {}", e))?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
    }

    Ok(downloaded)
}

/// 验证文件 SHA1
pub fn verify_sha1(path: &Path, expected: &str) -> Result<(), String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut hasher = sha1::Sha1::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("读取失败: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let result = hasher.finalize();
    let actual = format!("{:x}", result);
    if actual != expected {
        return Err(format!(
            "SHA1 不匹配: 期望 {}, 实际 {}",
            expected, actual
        ));
    }
    Ok(())
}

/// 解压 zip 文件
pub fn unzip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| format!("打开 zip 失败: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("解析 zip 失败: {}", e))?;
    archive
        .extract(dest)
        .map_err(|e| format!("解压失败: {}", e))?;
    Ok(())
}
