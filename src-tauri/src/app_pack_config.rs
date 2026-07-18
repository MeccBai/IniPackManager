fn parse_pack_config(pack_path: &Path) -> Result<RawPackConfig, String> {
    let config_path = pack_path.join("config.toml");
    if !config_path.exists() {
        return Err(format!("未找到 Pack 配置文件: {}", config_path.display()));
    }

    let raw = fs::read_to_string(&config_path)
        .map_err(|err| format!("读取 Pack 配置失败 {}: {err}", config_path.display()))?;
    toml::from_str(&raw).map_err(|err| format!("解析 Pack 配置失败 {}: {err}", config_path.display()))
}

fn read_pack_config_from_zip(zip_path: &Path) -> Result<(RawPackConfig, String), String> {
    let file = File::open(zip_path)
        .map_err(|err| format!("无法打开 zip 文件 {}: {err}", zip_path.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|err| format!("读取 zip 失败 {}: {err}", zip_path.display()))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| format!("读取 zip 条目失败: {err}"))?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().replace('\\', "/");
        if !entry_name.to_lowercase().ends_with("config.toml") {
            continue;
        }

        let mut raw = String::new();
        entry
            .read_to_string(&mut raw)
            .map_err(|err| format!("读取 zip 内 config.toml 失败: {err}"))?;
        let parsed: RawPackConfig =
            toml::from_str(&raw).map_err(|err| format!("解析 zip 内 config.toml 失败: {err}"))?;

        let prefix = entry_name
            .strip_suffix("config.toml")
            .unwrap_or("")
            .to_string();
        return Ok((parsed, prefix));
    }

    Err("zip 中未找到 config.toml".to_string())
}

fn pack_output_base(instance_path: &Path, pack_meta: &RawPackMeta) -> PathBuf {
    let mut path = instance_path.join("Pack");
    let clean_dir = pack_meta
        .dir
        .trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string();
    if !clean_dir.is_empty() {
        for part in clean_dir.split('/') {
            if !part.trim().is_empty() {
                path = path.join(part);
            }
        }
    }
    path
}

fn pack_dir_placeholder(pack_meta: &RawPackMeta) -> String {
    let output_dir = pack_output_base(Path::new(""), pack_meta);
    output_dir.to_string_lossy().replace('\\', "/")
}

fn first_toml_array_item(value: &toml::Value) -> Option<&toml::Value> {
    if let Some(array) = value.as_array() {
        return array.first();
    }
    Some(value)
}

fn option_default_bool(option: &RawPackOption) -> Option<bool> {
    option
        .default
        .as_ref()
        .and_then(first_toml_array_item)
        .and_then(|value| value.as_bool())
}

fn option_default_int(option: &RawPackOption) -> Option<i64> {
    option
        .default
        .as_ref()
        .and_then(first_toml_array_item)
        .and_then(|value| value.as_integer())
}

fn option_default_enum_index(option: &RawPackOption) -> Option<usize> {
    let values = &option.values;
    let value = option.default.as_ref().and_then(first_toml_array_item)?;
    if let Some(index) = value.as_integer() {
        return Some(index.max(0) as usize);
    }
    value
        .as_str()
        .and_then(|name| values.iter().position(|item| item == name))
}

fn toml_default_to_json(value: &toml::Value) -> Option<serde_json::Value> {
    let first = first_toml_array_item(value)?;
    if let Some(v) = first.as_bool() {
        return Some(serde_json::Value::Bool(v));
    }
    if let Some(v) = first.as_integer() {
        return Some(serde_json::Value::Number(v.into()));
    }
    if let Some(v) = first.as_str() {
        return Some(serde_json::Value::String(v.to_string()));
    }
    None
}

fn build_pack_definition(pack_path: &Path, config: &RawPackConfig) -> Result<PackDefinition, String> {
    let mut options = Vec::new();

    for option in &config.options {
        let option_type = option.option_type.to_lowercase();
        let display_desc = if option.desc.trim().is_empty() {
            option.name.clone()
        } else {
            option.desc.clone()
        };

        let default_bool = option_default_bool(option);
        let default_int = option_default_int(option);
        let default_enum_index = option_default_enum_index(option);

        if option_type == "enum" && option.values.is_empty() {
            return Err(format!("选项 {} 是 enum，但未提供 values", option.name));
        }

        options.push(PackOptionDefinition {
            name: option.name.clone(),
            desc: display_desc,
            option_type,
            placeholder: option.placeholders.first().cloned().unwrap_or_default(),
            default_bool,
            default_int,
            min: option.min,
            max: option.max,
            enum_items: option.values.clone(),
            default_enum_index,
        });
    }

    Ok(PackDefinition {
        pack_path: simplify_for_display(pack_path.to_path_buf()),
        name: config.config.name.clone(),
        desc: config.config.desc.clone(),
        dir: config.config.dir.clone(),
        config_id: config.config.id.trim().to_string(),
        version: config.config.version,
        requirements: PackRequirementDefinition {
            files: config
                .requirements
                .files
                .iter()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect(),
            pack: config
                .requirements
                .pack
                .iter()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect(),
            min_version: config
                .requirements
                .min_version
                .as_ref()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty()),
        },
        options,
    })
}

fn normalized_pack_config_id(raw_id: &str) -> String {
    raw_id.trim().to_ascii_lowercase()
}

fn effective_component_config_id(component: &ComponentState) -> String {
    let from_state = normalized_pack_config_id(&component.config_id);
    if !from_state.is_empty() {
        return from_state;
    }
    let pack_path = component.pack_path.trim();
    if pack_path.is_empty() {
        return String::new();
    }
    let parsed = parse_pack_config(Path::new(pack_path));
    match parsed {
        Ok(config) => normalized_pack_config_id(&config.config.id),
        Err(_) => String::new(),
    }
}

fn current_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn validate_min_app_version(config: &RawPackConfig) -> Result<(), String> {
    let Some(required_raw) = config.requirements.min_version.as_deref() else {
        return Ok(());
    };
    let required_text = required_raw.trim();
    if required_text.is_empty() {
        return Ok(());
    }

    let required = Version::parse(required_text)
        .map_err(|err| format!("Requirements.MinVersion 无效: {required_text} ({err})"))?;
    let current_text = current_app_version();
    let current = Version::parse(current_text)
        .map_err(|err| format!("当前应用版本无效: {current_text} ({err})"))?;

    if current < required {
        return Err(format!(
            "当前应用版本 {} 低于组件要求的最低版本 {}，无法安装或启用",
            current, required
        ));
    }

    Ok(())
}

fn validate_pack_requirements(
    instance_dir: &Path,
    instance_preset_id: &str,
    config: &RawPackConfig,
    all_components: &[ComponentState],
    current_component_id: Option<&str>,
) -> Result<(), String> {
    validate_min_app_version(config)?;

    let required_game = config.config.game.trim();
    if !required_game.is_empty() {
        let actual_game = instance_preset_id.trim();
        if !required_game.eq_ignore_ascii_case(actual_game) {
            return Err(format!(
                "组件游戏不匹配：需要 {}，当前实例是 {}",
                required_game,
                if actual_game.is_empty() { "(未设置)" } else { actual_game }
            ));
        }
    }

    let required_files: Vec<String> = config
        .requirements
        .files
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();
    let required_packs: Vec<String> = config
        .requirements
        .pack
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();

    let mut missing_files = Vec::new();
    for file in &required_files {
        let file_path = instance_dir.join(file);
        if !file_path.exists() {
            missing_files.push(file.clone());
        }
    }

    let current_component_id_norm = current_component_id.unwrap_or_default().trim().to_string();
    let mut enabled_pack_ids = HashSet::new();
    for component in all_components {
        if !component.enabled {
            continue;
        }
        if !current_component_id_norm.is_empty() && component.id.trim() == current_component_id_norm {
            continue;
        }
        let config_id = effective_component_config_id(component);
        if !config_id.is_empty() {
            enabled_pack_ids.insert(config_id);
        }
    }

    let mut missing_packs = Vec::new();
    for required in &required_packs {
        let normalized = normalized_pack_config_id(required);
        if normalized.is_empty() {
            continue;
        }
        if !enabled_pack_ids.contains(&normalized) {
            missing_packs.push(required.clone());
        }
    }

    if missing_files.is_empty() && missing_packs.is_empty() {
        return Ok(());
    }

    let mut parts = Vec::new();
    if !missing_files.is_empty() {
        parts.push(format!("缺少文件: {}", missing_files.join(", ")));
    }
    if !missing_packs.is_empty() {
        parts.push(format!("缺少依赖 Pack: {}", missing_packs.join(", ")));
    }

    Err(format!("依赖检查失败：{}", parts.join("；")))
}

fn instance_preset_id_for_path(instance_path: &str) -> Result<String, String> {
    let key = normalize_path_key(instance_path);
    let store_path = instance_store_path()?;
    let store = load_instance_store(&store_path)?;
    let preset = store
        .instances
        .iter()
        .find(|instance| normalize_path_key(&instance.path) == key)
        .map(|instance| instance.preset_id.clone())
        .unwrap_or_default();
    Ok(preset)
}
