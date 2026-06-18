fn read_preset_summary_from_dir(preset_dir: &Path) -> Result<PresetSummary, String> {
    let entry_path = preset_dir.join("entry.toml");
    if !entry_path.exists() {
        return Err(format!("缺少 preset 入口文件: {}", entry_path.display()));
    }
    let raw = fs::read_to_string(&entry_path)
        .map_err(|err| format!("读取 preset 入口失败 {}: {err}", entry_path.display()))?;
    let parsed: toml::Value = toml::from_str(&raw)
        .map_err(|err| format!("解析 preset 入口失败 {}: {err}", entry_path.display()))?;

    let info = parsed
        .get("Info")
        .and_then(|v| v.as_table())
        .ok_or_else(|| format!("preset 入口缺少 [Info] 段: {}", entry_path.display()))?;

    let id = info
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("preset 入口缺少 Info.id: {}", entry_path.display()))?
        .to_string();

    let name = info
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(&id)
        .to_string();

    Ok(PresetSummary {
        id,
        name,
        path: simplify_for_display(preset_dir.to_path_buf()),
    })
}

fn collect_preset_payload_relative_paths(
    preset_root: &Path,
    current_dir: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(current_dir)
        .map_err(|err| format!("读取 preset 目录失败 {}: {err}", current_dir.display()))?;

    for entry in entries {
        let entry = entry
            .map_err(|err| format!("读取 preset 文件失败 {}: {err}", current_dir.display()))?;
        let src_path = entry.path();
        let rel_path = src_path
            .strip_prefix(preset_root)
            .map_err(|err| format!("计算 preset 相对路径失败 {}: {err}", src_path.display()))?
            .to_path_buf();

        if rel_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("entry.toml"))
            .unwrap_or(false)
        {
            continue;
        }

        if src_path.is_dir() {
            collect_preset_payload_relative_paths(preset_root, &src_path, out)?;
        } else if src_path.is_file() {
            out.push(rel_path);
        }
    }

    Ok(())
}

fn detect_preset_overwrite_files(instance_dir: &Path, preset: &PresetSummary) -> Result<Vec<String>, String> {
    let preset_dir = Path::new(&preset.path);
    if !preset_dir.exists() {
        return Err(format!("preset 目录不存在: {}", preset.path));
    }

    let mut rel_paths = Vec::new();
    collect_preset_payload_relative_paths(preset_dir, preset_dir, &mut rel_paths)?;

    let mut overwrite_files = Vec::new();
    for rel_path in rel_paths {
        let dst_path = instance_dir.join(&rel_path);
        if dst_path.exists() {
            overwrite_files.push(rel_path.to_string_lossy().replace('\\', "/"));
        }
    }
    overwrite_files.sort();
    Ok(overwrite_files)
}

fn copy_preset_payload_recursive(
    preset_root: &Path,
    current_dir: &Path,
    instance_dir: &Path,
    copied: &mut Vec<String>,
) -> Result<(), String> {
    let entries = fs::read_dir(current_dir)
        .map_err(|err| format!("读取 preset 目录失败 {}: {err}", current_dir.display()))?;

    for entry in entries {
        let entry = entry
            .map_err(|err| format!("读取 preset 文件失败 {}: {err}", current_dir.display()))?;
        let src_path = entry.path();
        let rel_path = src_path
            .strip_prefix(preset_root)
            .map_err(|err| format!("计算 preset 相对路径失败 {}: {err}", src_path.display()))?;

        if rel_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("entry.toml"))
            .unwrap_or(false)
        {
            continue;
        }

        let dst_path = instance_dir.join(rel_path);
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)
                .map_err(|err| format!("创建目录失败 {}: {err}", dst_path.display()))?;
            copy_preset_payload_recursive(preset_root, &src_path, instance_dir, copied)?;
        } else if src_path.is_file() {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("创建目录失败 {}: {err}", parent.display()))?;
            }
            fs::copy(&src_path, &dst_path).map_err(|err| {
                format!(
                    "复制 preset 文件失败 {} -> {}: {err}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
            copied.push(format!(
                "{} -> {} (exists={})",
                src_path.display(),
                dst_path.display(),
                dst_path.exists()
            ));
        }
    }

    Ok(())
}

fn list_presets_internal() -> Result<Vec<PresetSummary>, String> {
    let root = presets_root_dir()?;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut presets = Vec::new();
    let entries = fs::read_dir(&root).map_err(|err| format!("读取 preset 目录失败 {}: {err}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("读取 preset 子目录失败 {}: {err}", root.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(summary) = read_preset_summary_from_dir(&path) {
            presets.push(summary);
        }
    }
    presets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(presets)
}

fn find_preset_by_id(preset_id: &str) -> Result<PresetSummary, String> {
    let presets = list_presets_internal()?;
    presets
        .into_iter()
        .find(|preset| preset.id == preset_id)
        .ok_or_else(|| format!("未找到 preset: {}", preset_id))
}

fn apply_preset_to_instance(instance_dir: &Path, preset: &PresetSummary) -> Result<(), String> {
    let preset_dir = Path::new(&preset.path);
    if !preset_dir.exists() {
        return Err(format!("preset 目录不存在: {}", preset.path));
    }

    let mut copied: Vec<String> = Vec::new();
    copy_preset_payload_recursive(preset_dir, preset_dir, instance_dir, &mut copied)?;

    let debug_log = instance_dir.join("IniPackManager.preset.log");
    let mut lines = Vec::new();
    lines.push(format!("preset_id={}", preset.id));
    lines.push(format!("preset_name={}", preset.name));
    lines.push(format!(
        "instance_dir={}",
        simplify_for_display(instance_dir.to_path_buf())
    ));
    lines.push(format!(
        "preset_dir={}",
        simplify_for_display(preset_dir.to_path_buf())
    ));
    let current_dir = std::env::current_dir()
        .map(simplify_for_display)
        .unwrap_or_else(|_| "(unknown)".to_string());
    lines.push(format!("current_dir={current_dir}"));
    lines.push(format!("copied_count={}", copied.len()));
    for file in copied {
        lines.push(format!("copied={}", file));
    }
    let mut content = lines.join("\n");
    content.push('\n');
    fs::write(&debug_log, content)
        .map_err(|err| format!("写入 preset 日志失败 {}: {err}", debug_log.display()))?;
    Ok(())
}
