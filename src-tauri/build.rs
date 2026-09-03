fn main() {
    // 注入构建期锁定的二进制版本与 SHA256，供 install.rs 运行时校验下载完整性。
    // 数据来自 scripts/fetch-binaries.ts 生成的 src-tauri/bin/binaries.sha256.json。
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR 未设置");
    let sha_path = std::path::Path::new(&manifest)
        .join("bin")
        .join("binaries.sha256.json");
    if let Ok(content) = std::fs::read_to_string(&sha_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            let emit = |key: &str, val: &serde_json::Value| {
                if let Some(s) = val.as_str() {
                    if !s.is_empty() {
                        println!("cargo:rustc-env={key}={s}");
                    }
                }
            };
            emit("VIDGRAB_YTDLP_VERSION", &v["yt-dlp"]["version"]);
            emit("VIDGRAB_YTDLP_WIN_SHA", &v["yt-dlp"]["win32"]);
            emit("VIDGRAB_YTDLP_MAC_SHA", &v["yt-dlp"]["darwin"]);
            emit("VIDGRAB_YTDLP_LINUX_SHA", &v["yt-dlp"]["linux"]);
            emit("VIDGRAB_FFMPEG_VERSION", &v["ffmpeg"]["version"]);
            emit("VIDGRAB_FFMPEG_DARWIN_ARM64_SHA", &v["ffmpeg"]["darwin-arm64"]);
            emit("VIDGRAB_FFMPEG_DARWIN_X64_SHA", &v["ffmpeg"]["darwin-x64"]);
            emit("VIDGRAB_FFMPEG_LINUX_ARM64_SHA", &v["ffmpeg"]["linux-arm64"]);
            emit("VIDGRAB_FFMPEG_LINUX_X64_SHA", &v["ffmpeg"]["linux-x64"]);
            emit("VIDGRAB_FFMPEG_WIN32_X64_SHA", &v["ffmpeg"]["win32-x64"]);
        } else {
            println!("cargo:warning=binaries.sha256.json 解析失败，安装器将不校验二进制；请重新运行 node scripts/fetch-binaries.ts");
        }
    } else {
        println!("cargo:warning=未找到 binaries.sha256.json，安装器将不校验二进制；请先运行 node scripts/fetch-binaries.ts");
    }
    // 哈希清单变化时重新注入编译期常量，避免 cargo 使用缓存的旧 build 结果
    println!("cargo:rerun-if-changed={}", sha_path.display());
    tauri_build::build()
}
