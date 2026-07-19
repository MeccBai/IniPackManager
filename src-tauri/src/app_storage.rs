fn instance_store_path() -> Result<PathBuf, String> {
    let user_store_path = user_home_dir()?.join(USER_INSTANCE_STORE_RELATIVE_PATH);

    if !user_store_path.exists() {
        let legacy_path = std::env::current_dir()
            .map(|dir| dir.join(LEGACY_INSTANCE_STORE_RELATIVE_PATH))
            .map_err(|err| format!("无法获取当前工作目录: {err}"))?;

        if legacy_path.exists() {
            ensure_parent_dir(&user_store_path)?;
            fs::copy(&legacy_path, &user_store_path).map_err(|err| {
                format!(
                    "迁移实例数据失败 {} -> {}: {err}",
                    legacy_path.display(),
                    user_store_path.display()
                )
            })?;
        }
    }

    Ok(user_store_path)
}

fn user_home_dir() -> Result<PathBuf, String> {
    let user_profile = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| "无法获取用户目录(USERPROFILE/HOME)".to_string())?;
    Ok(PathBuf::from(user_profile))
}

fn local_repository_base_dir() -> Result<PathBuf, String> {
    let settings = load_app_settings(&app_settings_store_path()?)?;
    let custom_path = settings.local_repository_path.trim();
    if !custom_path.is_empty() {
        return Ok(PathBuf::from(custom_path));
    }
    Ok(user_home_dir()?.join("IniPackManager"))
}

fn components_root_dir() -> Result<PathBuf, String> {
    Ok(local_repository_base_dir()?.join("components"))
}

fn repository_root_dir() -> Result<PathBuf, String> {
    Ok(local_repository_base_dir()?.join("repository"))
}

fn component_state_store_path() -> Result<PathBuf, String> {
    Ok(user_home_dir()?.join(USER_COMPONENT_STATE_RELATIVE_PATH))
}

fn app_settings_store_path() -> Result<PathBuf, String> {
    Ok(user_home_dir()?.join(USER_APP_SETTINGS_RELATIVE_PATH))
}

fn remote_index_cache_path() -> Result<PathBuf, String> {
    Ok(user_home_dir()?.join(USER_REMOTE_INDEX_CACHE_RELATIVE_PATH))
}

fn project_root_dir() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|err| format!("无法获取项目目录: {err}"))
}

fn presets_root_dir() -> Result<PathBuf, String> {
    let cwd = project_root_dir()?;
    let candidates = [
        cwd.join(PROJECT_PRESETS_RELATIVE_PATH),
        cwd.join("..").join(PROJECT_PRESETS_RELATIVE_PATH),
    ];

    for candidate in candidates {
        if candidate.exists() && candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Ok(cwd.join(PROJECT_PRESETS_RELATIVE_PATH))
}

fn sanitize_component_dir_name(name: &str) -> String {
    let invalid = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut sanitized = name.trim().to_string();
    for ch in invalid {
        sanitized = sanitized.replace(ch, "_");
    }
    sanitized = sanitized.trim_matches('.').trim().to_string();
    if sanitized.is_empty() {
        "Pack".to_string()
    } else {
        sanitized
    }
}

fn unique_component_dir(base_dir: PathBuf) -> PathBuf {
    if !base_dir.exists() {
        return base_dir;
    }

    let parent = base_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = base_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Pack")
        .to_string();

    for index in 2..=9999 {
        let candidate = parent.join(format!("{}_{}", stem, index));
        if !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{}_{}", stem, uuid_like_suffix()))
}

fn uuid_like_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}", duration.as_millis()),
        Err(_) => "ts".to_string(),
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|err| format!("无法创建数据目录 {}: {err}", parent.display()))
}

fn load_instance_store(path: &Path) -> Result<InstanceStore, String> {
    if !path.exists() {
        return Ok(InstanceStore::default());
    }

    let raw = fs::read_to_string(path)
        .map_err(|err| format!("读取实例数据失败 {}: {err}", path.display()))?;

    if raw.trim().is_empty() {
        return Ok(InstanceStore::default());
    }

    serde_json::from_str(&raw).map_err(|err| format!("解析实例数据失败 {}: {err}", path.display()))
}

fn save_instance_store(path: &Path, store: &InstanceStore) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let raw = serde_json::to_string_pretty(store)
        .map_err(|err| format!("序列化实例数据失败: {err}"))?;
    fs::write(path, raw).map_err(|err| format!("写入实例数据失败 {}: {err}", path.display()))
}

fn load_component_state_store(path: &Path) -> Result<ComponentStateStore, String> {
    if !path.exists() {
        return Ok(ComponentStateStore::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("读取组件状态失败 {}: {err}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(ComponentStateStore::default());
    }
    serde_json::from_str(&raw)
        .map_err(|err| format!("解析组件状态失败 {}: {err}", path.display()))
}

fn save_component_state_store(path: &Path, store: &ComponentStateStore) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let raw = serde_json::to_string_pretty(store)
        .map_err(|err| format!("序列化组件状态失败: {err}"))?;
    fs::write(path, raw).map_err(|err| format!("写入组件状态失败 {}: {err}", path.display()))
}

fn load_app_settings(path: &Path) -> Result<AppSettings, String> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("读取全局设置失败 {}: {err}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(AppSettings::default());
    }
    serde_json::from_str(&raw)
        .map_err(|err| format!("解析全局设置失败 {}: {err}", path.display()))
}

fn save_app_settings(path: &Path, settings: &AppSettings) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let raw = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("序列化全局设置失败: {err}"))?;
    fs::write(path, raw).map_err(|err| format!("写入全局设置失败 {}: {err}", path.display()))
}

fn simplify_for_display(path: PathBuf) -> String {
    let raw = path.to_string_lossy().to_string();

    #[cfg(windows)]
    {
        if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = raw.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
    }

    raw
}

fn normalize_path_key(raw_path: &str) -> String {
    let path = Path::new(raw_path);
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    let mut normalized = simplify_for_display(canonical);

    #[cfg(windows)]
    {
        normalized = normalized.replace('/', "\\").to_lowercase();
    }

    #[cfg(not(windows))]
    {
        normalized = normalized.replace('\\', "/");
    }

    normalized.trim_end_matches(['/', '\\']).to_string()
}

fn normalize_name(raw_name: &str) -> String {
    raw_name.trim().to_string()
}

fn fallback_name_from_path(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| path.to_string())
}

fn normalize_record_name(record: &mut InstanceRecord) {
    record.name = normalize_name(&record.name);
    if record.name.is_empty() {
        record.name = fallback_name_from_path(&record.path);
    }
}
