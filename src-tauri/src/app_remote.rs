const DEFAULT_REGISTRY_URL: &str = "https://raw.githubusercontent.com/MeccBai/IniPackRepo/master/index.toml";

fn load_effective_app_settings() -> Result<AppSettings, String> {
    let path = app_settings_store_path()?;
    let mut settings = load_app_settings(&path)?;
    if settings.registry_url.trim().is_empty() {
        settings.registry_url = DEFAULT_REGISTRY_URL.to_string();
    }
    Ok(settings)
}

fn save_effective_app_settings(settings: AppSettings) -> Result<AppSettings, String> {
    let mut next = settings;
    next.registry_url = next.registry_url.trim().to_string();
    next.local_repository_path = next.local_repository_path.trim().to_string();
    if next.registry_url.is_empty() {
        next.registry_url = DEFAULT_REGISTRY_URL.to_string();
    }
    let path = app_settings_store_path()?;
    save_app_settings(&path, &next)?;
    Ok(next)
}

fn current_app_version_text() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn parse_version_text(raw: &str) -> Result<Version, String> {
    Version::parse(raw).map_err(|err| format!("版本号无效: {raw} ({err})"))
}

fn package_incompatible_reason(min_version: &str) -> Option<String> {
    let required_text = min_version.trim();
    if required_text.is_empty() {
        return None;
    }
    let required = parse_version_text(required_text).ok()?;
    let current = parse_version_text(current_app_version_text()).ok()?;
    if current < required {
        return Some(format!("需要管理器版本 {} 或更高", required));
    }
    None
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .build()
        .map_err(|err| format!("初始化网络客户端失败: {err}"))
}

fn fetch_text(url: &str) -> Result<String, String> {
    let response = http_client()?
        .get(url)
        .send()
        .map_err(|err| format!("请求失败 {url}: {err}"))?
        .error_for_status()
        .map_err(|err| format!("请求返回错误 {url}: {err}"))?;
    response
        .text()
        .map_err(|err| format!("读取响应失败 {url}: {err}"))
}

fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = http_client()?
        .get(url)
        .send()
        .map_err(|err| format!("下载失败 {url}: {err}"))?
        .error_for_status()
        .map_err(|err| format!("下载返回错误 {url}: {err}"))?;
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|err| format!("读取下载内容失败 {url}: {err}"))
}

fn resolve_remote_url(base_url: &str, relative_or_absolute: &str) -> Result<String, String> {
    let text = relative_or_absolute.trim();
    if text.is_empty() {
        return Err("仓库索引中存在空路径".to_string());
    }
    let base = reqwest::Url::parse(base_url)
        .map_err(|err| format!("仓库 URL 无效 {base_url}: {err}"))?;
    base.join(text)
        .map(|url| url.to_string())
        .map_err(|err| format!("拼接仓库 URL 失败 {base_url} + {text}: {err}"))
}

fn normalize_sha256(raw: &str) -> String {
    raw.trim()
        .strip_prefix("sha256:")
        .unwrap_or(raw.trim())
        .trim()
        .to_ascii_lowercase()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn verify_download_sha256(bytes: &[u8], expected: &str) -> Result<(), String> {
    let normalized = normalize_sha256(expected);
    if normalized.is_empty() {
        return Ok(());
    }
    let actual = sha256_hex(bytes);
    if actual != normalized {
        return Err(format!(
            "下载文件校验失败：期望 sha256={}，实际 sha256={}",
            normalized, actual
        ));
    }
    Ok(())
}

fn temp_zip_path() -> std::path::PathBuf {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("ini_pack_manager_remote_{millis}.zip"))
}

fn import_pack_archive(zip_path: &Path) -> Result<PackDefinition, String> {
    let (parsed_config, config_prefix) = read_pack_config_from_zip(zip_path)?;
    validate_min_app_version(&parsed_config)?;

    let components_root = components_root_dir()?;
    fs::create_dir_all(&components_root)
        .map_err(|err| format!("无法创建组件目录 {}: {err}", components_root.display()))?;

    let name_from_config = sanitize_component_dir_name(&parsed_config.config.name);
    let zip_stem = zip_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(sanitize_component_dir_name)
        .unwrap_or_else(|| "Pack".to_string());
    let base_name = if name_from_config.is_empty() {
        zip_stem
    } else {
        name_from_config
    };

    let repository_root = repository_root_dir()?;
    fs::create_dir_all(&repository_root)
        .map_err(|err| format!("无法创建中央仓库目录 {}: {err}", repository_root.display()))?;
    let normalized_id = parsed_config.config.id.trim().to_string();

    let (target_dir, repo_target) = if !normalized_id.is_empty() {
        let mut found_component_dir = None;
        let mut found_repo_dir = None;

        if components_root.exists() {
            for entry in fs::read_dir(&components_root)
                .map_err(|err| format!("读取组件目录失败 {}: {err}", components_root.display()))?
            {
                let path = entry
                    .map_err(|err| format!("读取组件目录项失败: {err}"))?
                    .path();
                if path.is_dir()
                    && parse_pack_config(&path)
                        .is_ok_and(|config| config.config.id.trim().eq_ignore_ascii_case(&normalized_id))
                {
                    found_component_dir = Some(path);
                    break;
                }
            }
        }

        if repository_root.exists() {
            for entry in fs::read_dir(&repository_root)
                .map_err(|err| format!("读取中央仓库目录失败 {}: {err}", repository_root.display()))?
            {
                let path = entry
                    .map_err(|err| format!("读取中央仓库目录项失败: {err}"))?
                    .path();
                if path.is_dir()
                    && parse_pack_config(&path)
                        .is_ok_and(|config| config.config.id.trim().eq_ignore_ascii_case(&normalized_id))
                {
                    found_repo_dir = Some(path);
                    break;
                }
            }
        }

        let component_target = found_component_dir
            .unwrap_or_else(|| unique_component_dir(components_root.join(&base_name)));
        let repo_target = found_repo_dir.unwrap_or_else(|| {
            unique_component_dir(
                repository_root.join(
                    component_target
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("Pack"),
                ),
            )
        });
        (component_target, repo_target)
    } else {
        let target_dir = unique_component_dir(components_root.join(base_name));
        let repo_target = unique_component_dir(
            repository_root.join(
                target_dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Pack"),
            ),
        );
        (target_dir, repo_target)
    };

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)
            .map_err(|err| format!("覆盖组件目录失败 {}: {err}", target_dir.display()))?;
    }
    fs::create_dir_all(&target_dir)
        .map_err(|err| format!("无法创建组件目录 {}: {err}", target_dir.display()))?;
    extract_zip_to_directory(zip_path, &target_dir, &config_prefix)?;

    if repo_target.exists() {
        fs::remove_dir_all(&repo_target)
            .map_err(|err| format!("覆盖中央仓库目录失败 {}: {err}", repo_target.display()))?;
    }
    copy_dir_recursive(&target_dir, &repo_target)?;

    build_pack_definition(&repo_target, &parsed_config)
}

fn load_remote_package_catalog(registry_url: &str, game: &str) -> Result<RemotePackageCatalog, String> {
    let index_url = registry_url.trim();
    if index_url.is_empty() {
        return Err("中央仓库地址不能为空".to_string());
    }
    let game_text = game.trim();
    if game_text.is_empty() {
        return Err("当前实例缺少 Preset/Game，无法匹配云端仓库".to_string());
    }

    let index_raw = fetch_text(index_url)?;
    let index: RemoteRegistryIndex = toml::from_str(&index_raw)
        .map_err(|err| format!("解析中央仓库索引失败 {}: {err}", index_url))?;

    let mut catalog_name = String::new();
    let mut catalog_desc = String::new();
    let mut list_urls = Vec::new();
    for pack_list in index.pack_lists {
        if pack_list.game.trim().eq_ignore_ascii_case(game_text) {
            if catalog_name.is_empty() {
                catalog_name = pack_list.name.trim().to_string();
            }
            if catalog_desc.is_empty() {
                catalog_desc = pack_list.desc.trim().to_string();
            }
            for item in pack_list.index {
                list_urls.push(resolve_remote_url(index_url, &item)?);
            }
        }
    }

    let mut packages = Vec::new();
    for list_url in list_urls {
        let list_raw = fetch_text(&list_url)?;
        let list_file: RemotePackageListFile = toml::from_str(&list_raw)
            .map_err(|err| format!("解析包列表失败 {}: {err}", list_url))?;
        for mut package in list_file.packages {
            package.url = resolve_remote_url(&list_url, &package.url)?;
            packages.push(package);
        }
    }

    Ok(RemotePackageCatalog {
        game: game_text.to_string(),
        name: catalog_name,
        desc: catalog_desc,
        packages: packages
            .into_iter()
            .map(|package| RemotePackageSummary {
                id: package.id.trim().to_string(),
                name: package.name.trim().to_string(),
                author: package.author.trim().to_string(),
                desc: package.desc.trim().to_string(),
                version: package.version,
                url: package.url.trim().to_string(),
                sha256: package.sha256.trim().to_string(),
                min_version: package.min_version.trim().to_string(),
                incompatible_reason: package_incompatible_reason(&package.min_version),
            })
            .collect(),
    })
}

#[tauri::command]
fn get_app_settings() -> Result<AppSettings, String> {
    load_effective_app_settings()
}

#[tauri::command]
fn save_app_settings_command(settings: AppSettings) -> Result<AppSettings, String> {
    save_effective_app_settings(settings)
}

#[tauri::command]
fn list_remote_packages(input: LoadRemotePackagesInput) -> Result<RemotePackageCatalog, String> {
    load_remote_package_catalog(&input.registry_url, &input.game)
}

#[tauri::command]
fn import_remote_package(input: ImportRemotePackageInput) -> Result<PackDefinition, String> {
    let url = input.url.trim();
    if url.is_empty() {
        return Err("下载地址不能为空".to_string());
    }
    let bytes = fetch_bytes(url)?;
    if let Some(expected) = input.sha256.as_deref() {
        verify_download_sha256(&bytes, expected)?;
    }

    let temp_path = temp_zip_path();
    fs::write(&temp_path, bytes)
        .map_err(|err| format!("写入临时下载文件失败 {}: {err}", temp_path.display()))?;
    let imported = import_pack_archive(&temp_path);
    let _ = fs::remove_file(&temp_path);
    imported
}
