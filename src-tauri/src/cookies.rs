//! Cookie 固定存储（Netscape 格式），按站点组管理。
//! 用户添加一份 cookies.txt 即一个组，组名从文件里的域名推断（如「抖音」）。
//! 分组标记写在注释行，yt-dlp 忽略注释行，不影响读取。
//! 删除以组为单位：一次删掉整站 cookie，用户无须关心文件里具体有哪些子域
//! （页面会带出 CDN、埋点等无关域名，暴露它们只会造成心智负担）。

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager};

/// 分组标记前缀（Netscape 格式允许注释行，yt-dlp 会忽略）
const GROUP_PREFIX: &str = "# @group:";
/// Netscape cookie 文件头，yt-dlp 依赖它识别格式。
const COOKIE_HEADER: &str = "# Netscape HTTP Cookie File";
/// HttpOnly cookie 的行前缀，剥离后才是真实的 domain 字段。
const HTTPONLY_PREFIX: &str = "#HttpOnly_";

/// 单条 cookie 的唯一键：同域名 + 同路径 + 同名视为同一条。
#[derive(PartialEq, Eq, Clone)]
struct CookieKey {
    domain: String,
    path: String,
    name: String,
}

/// 解析出的一个 cookie 条目；raw 保留原始行（含可能的 #HttpOnly_ 前缀）。
struct CookieEntry {
    key: CookieKey,
    raw: String,
    /// 所属站点组名，空串表示早期无分组标记的数据
    group: String,
}

/// 去掉 UTF-8 BOM：带 BOM 的文件会被 yt-dlp 判为「不是 Netscape 格式」而失败。
fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

/// 解析 Netscape cookie 文本：跳过空行与注释行，字段数不为 7 的行忽略。
/// `# @group:` 之后的条目归入该组，直到遇到下一个分组标记。
fn parse_entries(text: &str) -> Vec<CookieEntry> {
    let mut out: Vec<CookieEntry> = Vec::new();
    let mut group = String::new();
    for line in strip_bom(text).lines() {
        let line = line.trim_end_matches('\r');
        if let Some(name) = line.strip_prefix(GROUP_PREFIX) {
            group = name.trim().to_string();
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        // #HttpOnly_ 开头是真实 cookie，其余 # 开头是注释
        let is_httponly = line.starts_with(HTTPONLY_PREFIX);
        if line.starts_with('#') && !is_httponly {
            continue;
        }
        let body = if is_httponly {
            &line[HTTPONLY_PREFIX.len()..]
        } else {
            line
        };
        let cols: Vec<&str> = body.split('\t').collect();
        if cols.len() != 7 {
            continue;
        }
        out.push(CookieEntry {
            key: CookieKey {
                domain: cols[0].to_string(),
                path: cols[2].to_string(),
                name: cols[5].to_string(),
            },
            raw: line.to_string(),
            group: group.clone(),
        });
    }
    out
}

/// 按组分段落盘：每组前写一个分组标记，组内条目紧随其后。
fn serialize_entries(entries: &[CookieEntry]) -> String {
    let mut out = String::from(COOKIE_HEADER);
    out.push('\n');
    let mut order: Vec<&str> = Vec::new();
    for e in entries {
        if !order.contains(&e.group.as_str()) {
            order.push(e.group.as_str());
        }
    }
    for g in order {
        if !g.is_empty() {
            out.push_str(GROUP_PREFIX);
            out.push_str(g);
            out.push('\n');
        }
        for e in entries.iter().filter(|e| e.group == g) {
            out.push_str(&e.raw);
            out.push('\n');
        }
    }
    out
}

/// 同组内按（域名 + 路径 + 名）去重，保留最后一条（后导入的覆盖先导入的）。
fn dedup(entries: Vec<CookieEntry>) -> Vec<CookieEntry> {
    let mut seen: HashSet<(String, String, String, String)> = HashSet::new();
    let mut out: VecDeque<CookieEntry> = VecDeque::new();
    for e in entries.into_iter().rev() {
        let k = (
            e.group.clone(),
            e.key.domain.clone(),
            e.key.path.clone(),
            e.key.name.clone(),
        );
        if seen.insert(k) {
            out.push_front(e);
        }
    }
    out.into_iter().collect()
}

/// 合并进指定组：该组旧条目整体替换，其余组原样保留。
fn merge_into_group(base: Vec<CookieEntry>, source: Vec<CookieEntry>, group: &str) -> Vec<CookieEntry> {
    let mut out: Vec<CookieEntry> = base.into_iter().filter(|e| e.group != group).collect();
    out.extend(source);
    dedup(out)
}

/// 取主域（eTLD+1 的简化版）：去掉子域前缀，保留可标识站点的部分。
/// 用于兜底组名与展示，不追求覆盖全部公共后缀。
fn registrable_domain(domain: &str) -> String {
    let d = domain.trim_start_matches('.').to_lowercase();
    let parts: Vec<&str> = d.split('.').collect();
    if parts.len() <= 2 {
        return d;
    }
    let last2 = parts[parts.len() - 2..].join(".");
    // 双段后缀（com.cn 等）：只取两段会得到无意义的 "com.cn"，需多取一段
    const TWO_LEVEL: [&str; 8] = [
        "com.cn", "net.cn", "org.cn", "gov.cn", "edu.cn", "co.uk", "com.hk", "co.jp",
    ];
    if TWO_LEVEL.contains(&last2.as_str()) {
        return parts[parts.len() - 3..].join(".");
    }
    last2
}

/// 推断组名：优先用域名能识别出的平台名（抖音/哔哩哔哩…），否则用第一个主域兜底。
fn infer_group_name(entries: &[CookieEntry]) -> String {
    let mut domains: Vec<String> = Vec::new();
    for e in entries {
        let d = registrable_domain(&e.key.domain);
        if !d.is_empty() && !domains.contains(&d) {
            domains.push(d);
        }
    }
    for d in &domains {
        let p = crate::url::detect_platform(&format!("https://{}", d));
        if p != "未知平台" {
            return p.to_string();
        }
    }
    domains.into_iter().next().unwrap_or_default()
}

/// 列出当前所有站点组（按存储顺序），供设置界面展示与整组删除。
fn groups_of(entries: &[CookieEntry]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for e in entries {
        if e.group.is_empty() {
            continue;
        }
        if !out.contains(&e.group) {
            out.push(e.group.clone());
        }
    }
    out
}

/// 读取文本文件并去掉 BOM
fn read_text(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读取 Cookie 文件失败: {}", e))?;
    Ok(strip_bom(&String::from_utf8_lossy(&bytes)).to_string())
}

/// 固定存储文件路径：app_data/cookies.txt，前端经 cookie_store_path 取后作为 --cookies 参数传回。
fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    Ok(dir.join("cookies.txt"))
}

/// 固定存储的概览：已启用的站点组。groups 非空即视为已启用。
#[derive(Serialize)]
pub struct CookieStoreStatus {
    pub groups: Vec<String>,
}

/// 固定存储的实际路径，供前端作为 --cookies 参数值。
#[tauri::command]
pub async fn cookie_store_path(app: AppHandle) -> Result<String, String> {
    store_path(&app).map(|p| p.to_string_lossy().to_string())
}

/// 固定存储状态（站点组列表），供设置界面展示。
#[tauri::command]
pub async fn cookie_store_status(app: AppHandle) -> Result<CookieStoreStatus, String> {
    let path = store_path(&app)?;
    if !path.is_file() {
        return Ok(CookieStoreStatus { groups: Vec::new() });
    }
    let text = read_text(&path.to_string_lossy())?;
    Ok(CookieStoreStatus {
        groups: groups_of(&parse_entries(&text)),
    })
}

/// 把用户导出的 cookies.txt 合并进固定存储（不存在则创建），返回该组名。
/// 同组再次添加即整站替换（重新导出后不残留过期条目）。
#[tauri::command]
pub async fn add_cookie_store(app: AppHandle, source: String) -> Result<String, String> {
    if source.is_empty() {
        return Err("未选择 Cookie 文件".to_string());
    }
    // 防御性校验：只接受真实存在的 .txt 文件且大小合理，
    // 避免命令被（如被 XSS 注入的脚本）借机读取本机任意路径或超大文件。
    // 内容合法性仍由下方 parse_entries 兜底（解析不出合法 cookie 即拒绝）。
    let src_path = Path::new(&source);
    if !src_path.is_file() {
        return Err("Cookie 文件不存在".to_string());
    }
    if !source.to_lowercase().ends_with(".txt") {
        return Err("Cookie 文件必须是 .txt（Netscape 格式）".to_string());
    }
    if let Ok(meta) = std::fs::metadata(&source) {
        const MAX_COOKIE_FILE: u64 = 16 * 1024 * 1024; // 16MB
        if meta.len() > MAX_COOKIE_FILE {
            return Err("Cookie 文件过大（上限 16MB）".to_string());
        }
    }
    let source_text = read_text(&source)?;
    // 空文件或纯注释文件视为无效导出
    let mut incoming = parse_entries(&source_text);
    if incoming.is_empty() {
        return Err(
            "这个文件不是有效的 cookies.txt（Netscape 格式），请用浏览器扩展重新导出".to_string(),
        );
    }
    let group = infer_group_name(&incoming);
    if group.is_empty() {
        return Err("无法从该文件识别出站点，请确认导出的是有效的 cookies.txt".to_string());
    }
    for e in &mut incoming {
        e.group = group.clone();
    }

    let path = store_path(&app)?;
    let base = if path.is_file() {
        parse_entries(&read_text(&path.to_string_lossy())?)
    } else {
        Vec::new()
    };
    let merged = serialize_entries(&merge_into_group(base, incoming, &group));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    std::fs::write(&path, merged).map_err(|e| format!("写入 Cookie 存储失败: {}", e))?;
    Ok(group)
}

/// 删除指定站点组（整组：该组所有域名与条目一并移除）。
#[tauri::command]
pub async fn remove_cookie_group(app: AppHandle, group: String) -> Result<(), String> {
    let path = store_path(&app)?;
    if !path.is_file() {
        return Ok(());
    }
    let entries = parse_entries(&read_text(&path.to_string_lossy())?);
    let rest: Vec<CookieEntry> = entries.into_iter().filter(|e| e.group != group).collect();
    std::fs::write(&path, serialize_entries(&rest))
        .map_err(|e| format!("写入 Cookie 存储失败: {}", e))
}

/// 清空固定存储（直接删除文件）。
#[tauri::command]
pub async fn clear_cookie_store(app: AppHandle) -> Result<(), String> {
    let path = store_path(&app)?;
    match std::fs::remove_file(&path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("删除 Cookie 存储失败: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(domain: &str, name: &str, value: &str) -> String {
        format!("{}\tTRUE\t/\tFALSE\t0\t{}\t{}", domain, name, value)
    }

    fn text_of(lines: &[String]) -> String {
        let mut s = String::from(COOKIE_HEADER);
        s.push('\n');
        for l in lines {
            s.push_str(l);
            s.push('\n');
        }
        s
    }

    #[test]
    fn parse_entries_skips_comments_and_invalid_lines() {
        let text = format!(
            "{}\n# a comment\n\n{}\nbroken line\n",
            COOKIE_HEADER,
            line(".douyin.com", "a", "1")
        );
        let entries = parse_entries(&text);
        assert_eq!(entries.len(), 1, "应只保留一条合法 cookie");
        assert_eq!(entries[0].key.domain, ".douyin.com");
    }

    #[test]
    fn parse_entries_strips_bom() {
        let text = format!("\u{feff}{}\n{}", COOKIE_HEADER, line(".a.com", "k", "v"));
        assert_eq!(parse_entries(&text).len(), 1, "带 BOM 也应正常解析");
    }

    #[test]
    fn parse_entries_handles_httponly_prefix() {
        let text = format!("#HttpOnly_{}", line(".b.com", "k", "v"));
        let entries = parse_entries(&text);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key.domain, ".b.com", "HttpOnly 前缀应被剥离");
        assert!(entries[0].raw.starts_with(HTTPONLY_PREFIX), "原始行应保留前缀");
    }

    #[test]
    fn parse_entries_assigns_group_after_marker() {
        let text = format!(
            "{}抖音\n{}\n{}哔哩哔哩\n{}",
            GROUP_PREFIX,
            line(".douyin.com", "a", "1"),
            GROUP_PREFIX,
            line(".bilibili.com", "b", "2")
        );
        let entries = parse_entries(&text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].group, "抖音");
        assert_eq!(entries[1].group, "哔哩哔哩");
    }

    #[test]
    fn serialize_writes_group_markers_in_order() {
        let entries = vec![
            CookieEntry {
                key: CookieKey {
                    domain: ".douyin.com".into(),
                    path: "/".into(),
                    name: "a".into(),
                },
                raw: line(".douyin.com", "a", "1"),
                group: "抖音".into(),
            },
            CookieEntry {
                key: CookieKey {
                    domain: ".bilibili.com".into(),
                    path: "/".into(),
                    name: "b".into(),
                },
                raw: line(".bilibili.com", "b", "2"),
                group: "哔哩哔哩".into(),
            },
        ];
        let out = serialize_entries(&entries);
        assert!(out.starts_with(COOKIE_HEADER));
        assert!(out.contains(&format!("{}抖音", GROUP_PREFIX)));
        assert!(out.contains(&format!("{}哔哩哔哩", GROUP_PREFIX)));
        assert!(
            out.find("抖音").unwrap() < out.find("哔哩哔哩").unwrap(),
            "组顺序应保持存储顺序"
        );
    }

    #[test]
    fn merge_into_group_replaces_whole_group() {
        let base = parse_entries(&text_of(&[
            line(".douyin.com", "old_a", "1"),
            line(".douyin.com", "old_b", "2"),
        ]));
        let mut base: Vec<CookieEntry> = base
            .into_iter()
            .map(|mut e| {
                e.group = "抖音".into();
                e
            })
            .collect();
        let src: Vec<CookieEntry> = parse_entries(&text_of(&[line(".douyin.com", "new_a", "3")]))
            .into_iter()
            .map(|mut e| {
                e.group = "抖音".into();
                e
            })
            .collect();
        base = merge_into_group(base, src, "抖音");
        let out = serialize_entries(&base);
        assert_eq!(base.len(), 1, "同组旧条目应整体被替换");
        assert!(!out.contains("old_a") && !out.contains("old_b"));
    }

    #[test]
    fn merge_into_group_keeps_other_groups() {
        let douyin: Vec<CookieEntry> = parse_entries(&text_of(&[line(".douyin.com", "k", "1")]))
            .into_iter()
            .map(|mut e| {
                e.group = "抖音".into();
                e
            })
            .collect();
        let bili: Vec<CookieEntry> = parse_entries(&text_of(&[line(".bilibili.com", "k", "2")]))
            .into_iter()
            .map(|mut e| {
                e.group = "哔哩哔哩".into();
                e
            })
            .collect();
        let base = merge_into_group(Vec::new(), douyin, "抖音");
        let merged = merge_into_group(base, bili, "哔哩哔哩");
        assert_eq!(merged.len(), 2, "不同组应共存");
        assert_eq!(groups_of(&merged), vec!["抖音", "哔哩哔哩"]);
    }

    #[test]
    fn dedup_keeps_last_within_group() {
        let entries = vec![
            CookieEntry {
                key: CookieKey {
                    domain: ".douyin.com".into(),
                    path: "/".into(),
                    name: "a".into(),
                },
                raw: line(".douyin.com", "a", "1"),
                group: "抖音".into(),
            },
            CookieEntry {
                key: CookieKey {
                    domain: ".douyin.com".into(),
                    path: "/".into(),
                    name: "a".into(),
                },
                raw: line(".douyin.com", "a", "9"),
                group: "抖音".into(),
            },
        ];
        let out = dedup(entries);
        assert_eq!(out.len(), 1);
        assert!(out[0].raw.ends_with("9"), "应保留后导入的那条");
    }

    #[test]
    fn registrable_domain_strips_subdomain() {
        assert_eq!(registrable_domain("www.douyin.com"), "douyin.com");
        assert_eq!(registrable_domain(".douyin.com"), "douyin.com");
        assert_eq!(registrable_domain("v5-default.365yg.com"), "365yg.com");
        assert_eq!(registrable_domain("a.b.com.cn"), "b.com.cn");
    }

    #[test]
    fn infer_group_name_prefers_platform() {
        // 抖音页面会带出 365yg 等 CDN 域，仍应识别出「抖音」
        let entries = vec![
            CookieEntry {
                key: CookieKey {
                    domain: "v5-default.365yg.com".into(),
                    path: "/".into(),
                    name: "a".into(),
                },
                raw: line("v5-default.365yg.com", "a", "1"),
                group: String::new(),
            },
            CookieEntry {
                key: CookieKey {
                    domain: "www.douyin.com".into(),
                    path: "/".into(),
                    name: "b".into(),
                },
                raw: line("www.douyin.com", "b", "2"),
                group: String::new(),
            },
        ];
        assert_eq!(infer_group_name(&entries), "抖音");
    }

    #[test]
    fn infer_group_name_falls_back_to_domain() {
        let entries = vec![CookieEntry {
            key: CookieKey {
                domain: "www.example.com".into(),
                path: "/".into(),
                name: "a".into(),
            },
            raw: line("www.example.com", "a", "1"),
            group: String::new(),
        }];
        assert_eq!(infer_group_name(&entries), "example.com");
    }
}
