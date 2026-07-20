fn parse_pack_config(pack_path: &Path) -> Result<RawPackConfig, String> {
    let config_path = pack_path.join("config.toml");
    if !config_path.exists() {
        return Err(format!("未找到 Pack 配置文件: {}", config_path.display()));
    }

    let raw = fs::read_to_string(&config_path)
        .map_err(|err| format!("读取 Pack 配置失败 {}: {err}", config_path.display()))?;
    parse_pack_config_text(&raw, pack_path, &config_path)
}

fn parse_option_group(tag: &str, value: toml::Value, source: &Path) -> Result<RawPackOptionGroup, String> {
    if value
        .as_table()
        .is_some_and(|table| table.contains_key("Include"))
    {
        return Err(format!(
            "OptionGroup 不允许 Include，分组必须保持单层扁平: {}",
            source.display()
        ));
    }
    let mut group: RawPackOptionGroup = value
        .try_into()
        .map_err(|err| format!("解析选项组 {} 失败 {}: {err}", tag, source.display()))?;
    if group.options.is_empty() {
        return Err(format!("选项组 {} 未定义 Options: {}", tag, source.display()));
    }
    group.tag = tag.to_string();
    for option in &mut group.options {
        let name = option.name.trim();
        if name.is_empty() {
            return Err(format!("选项组 {} 存在空 Name: {}", tag, source.display()));
        }
        option.name = format!("{}.{}", tag, name);
        option.tag = tag.to_string();
    }
    Ok(group)
}

fn load_included_option_group(pack_path: &Path, include: &str) -> Result<RawPackOptionGroup, String> {
    let relative = safe_relative_path(include)
        .map_err(|_| format!("Include 路径无效，必须是包内相对路径: {include}"))?;
    let include_path = pack_path.join(relative);
    let raw = fs::read_to_string(&include_path)
        .map_err(|err| format!("读取 Include 文件失败 {}: {err}", include_path.display()))?;
    let document: toml::Value = toml::from_str(&raw)
        .map_err(|err| format!("解析 Include 文件失败 {}: {err}", include_path.display()))?;
    let table = document
        .as_table()
        .ok_or_else(|| format!("Include 文件必须定义一个 OptionGroup: {}", include_path.display()))?;
    if table.len() != 1 {
        return Err(format!(
            "Include 文件只能定义一个扁平 OptionGroup，且不允许递归 Include: {}",
            include_path.display()
        ));
    }
    let (tag, group_value) = table.iter().next().expect("table is not empty");
    parse_option_group(tag, group_value.clone(), &include_path)
}

fn parse_pack_config_text(raw: &str, pack_path: &Path, source: &Path) -> Result<RawPackConfig, String> {
    parse_pack_config_with_include_loader(raw, source, |include| {
        load_included_option_group(pack_path, include)
    })
}

fn normalize_pack_tag(raw: &str) -> Result<String, String> {
    let tag = raw.trim();
    if tag.is_empty() {
        return Ok("General".to_string());
    }
    PACK_TAGS
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(tag))
        .map(|(key, _)| (*key).to_string())
        .ok_or_else(|| format!("未知 Tag {tag}，请使用仓库定义的 Tag"))
}

fn load_pack_desc_html(pack_path: &Path, desc_file: &str) -> Result<Option<String>, String> {
    let file = desc_file.trim();
    if file.is_empty() {
        return Ok(None);
    }
    let relative = safe_relative_path(file)
        .map_err(|_| format!("DescFile 路径无效，必须是包内相对路径: {file}"))?;
    let extension = relative.extension().and_then(|extension| extension.to_str());
    if !extension.is_some_and(|extension| extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")) {
        return Err(format!("DescFile 必须指向 .html 文件: {file}"));
    }
    fs::read_to_string(pack_path.join(relative))
        .map(Some)
        .map_err(|err| format!("读取 DescFile 失败 {file}: {err}"))
}

fn dependency_display_names(requirements: &[String]) -> Vec<String> {
    let repository = repository_root_dir().ok();
    requirements
        .iter()
        .map(|requirement| {
            let name = requirement.trim();
            let Some(repository) = repository.as_ref() else {
                return name.to_string();
            };
            let Ok(entries) = fs::read_dir(repository) else {
                return name.to_string();
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if let Ok(config) = parse_pack_config(&path) {
                    if config.config.id.eq_ignore_ascii_case(name)
                        || config.config.name.eq_ignore_ascii_case(name)
                    {
                        return config.config.name;
                    }
                }
            }
            name.to_string()
        })
        .collect()
}

fn parse_pack_config_with_include_loader<F>(
    raw: &str,
    source: &Path,
    mut load_include: F,
) -> Result<RawPackConfig, String>
where
    F: FnMut(&str) -> Result<RawPackOptionGroup, String>,
{
    let mut config: RawPackConfig = toml::from_str(raw)
        .map_err(|err| format!("解析 Pack 配置失败 {}: {err}", source.display()))?;
    config.config.tag = normalize_pack_tag(&config.config.tag)
        .map_err(|err| format!("{}: {err}", source.display()))?;
    if config.config.option_groups.is_empty() {
        let mut options = config.options.clone();
        for option in &mut options {
            option.tag = "General".to_string();
        }
        config.options = options.clone();
        config.option_groups = vec![RawPackOptionGroup {
            tag: "General".to_string(),
            name: "General".to_string(),
            desc: String::new(),
            options,
        }];
        return Ok(config);
    }
    if !config.options.is_empty() {
        return Err(format!(
            "新版 OptionGroups 不能与传统 [[Options]] 混用: {}",
            source.display()
        ));
    }

    let document: toml::Value = toml::from_str(raw)
        .map_err(|err| format!("解析 Pack 配置失败 {}: {err}", source.display()))?;
    let table = document
        .as_table()
        .ok_or_else(|| format!("Pack 配置根节点必须是 TOML 表: {}", source.display()))?;
    let mut groups = HashMap::new();
    for tag in &config.config.option_groups {
        let tag = tag.trim();
        if tag.is_empty() || groups.contains_key(tag) {
            return Err(format!("OptionGroups 包含空或重复 Tag: {}", source.display()));
        }
        if let Some(value) = table.get(tag) {
            groups.insert(tag.to_string(), parse_option_group(tag, value.clone(), source)?);
        }
    }
    for include in &config.config.includes {
        let group = load_include(include)?;
        if !config.config.option_groups.iter().any(|tag| tag == &group.tag) {
            return Err(format!("Include 的 OptionGroup {} 未列在 Config.OptionGroups", group.tag));
        }
        if groups.insert(group.tag.clone(), group).is_some() {
            return Err(format!("OptionGroup 同时在主文件与 Include 中定义"));
        }
    }

    let mut normalized_groups = Vec::new();
    for tag in &config.config.option_groups {
        let group = groups
            .remove(tag.trim())
            .ok_or_else(|| format!("OptionGroups 声明的 {} 未定义", tag))?;
        normalized_groups.push(group);
    }
    config.options = normalized_groups
        .iter()
        .flat_map(|group| group.options.clone())
        .collect();
    config.option_groups = normalized_groups;
    Ok(config)
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
        let prefix = entry_name
            .strip_suffix("config.toml")
            .unwrap_or("")
            .to_string();
        drop(entry);
        let parsed = parse_pack_config_with_include_loader(
            &raw,
            Path::new(&entry_name),
            |include| {
                let relative = safe_relative_path(include).map_err(|_| {
                    format!("Include 路径无效，必须是包内相对路径: {include}")
                })?;
                let include_name = format!("{}{}", prefix, relative.to_string_lossy().replace('\\', "/"));
                let mut included = archive.by_name(&include_name).map_err(|err| {
                    format!("读取 zip Include 文件失败 {}: {err}", include_name)
                })?;
                let mut included_raw = String::new();
                included.read_to_string(&mut included_raw).map_err(|err| {
                    format!("读取 zip Include 内容失败 {}: {err}", include_name)
                })?;
                let document: toml::Value = toml::from_str(&included_raw).map_err(|err| {
                    format!("解析 zip Include 文件失败 {}: {err}", include_name)
                })?;
                let table = document.as_table().ok_or_else(|| {
                    format!("zip Include 文件必须定义一个 OptionGroup: {}", include_name)
                })?;
                if table.len() != 1 {
                    return Err(format!(
                        "zip Include 文件只能定义一个扁平 OptionGroup，且不允许递归 Include: {}",
                        include_name
                    ));
                }
                let (tag, group_value) = table.iter().next().expect("table is not empty");
                parse_option_group(tag, group_value.clone(), Path::new(&include_name))
            },
        )?;
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
        if option.control && option_type != "bool" && option_type != "enum" {
            return Err(format!(
                "控制选项 {} 只能使用 bool 或 enum 类型",
                option.name
            ));
        }
        let display_desc = if option.desc.trim().is_empty() {
            option.name.clone()
        } else {
            option.desc.clone()
        };

        let default_bool = option_default_bool(option);
        let default_int = option_default_int(option);
        let default_enum_index = option_default_enum_index(option);

        let enum_items = if option_type == "enum" {
            if option.values.is_empty() {
                return Err(format!("选项 {} 是 enum，但未提供 Values", option.name));
            }
            if !option.control {
                let placeholder_count = option_placeholders(option).len();
                if placeholder_count == 0 {
                    return Err(format!("选项 {} 缺少 placeholders", option.name));
                }
                enum_option_sets(option, placeholder_count)?;
            }
            option.values.clone()
        } else {
            Vec::new()
        };

        options.push(PackOptionDefinition {
            name: option.name.clone(),
            tag: option.tag.clone(),
            ui_name: option.ui_name.clone(),
            desc: display_desc,
            option_type,
            placeholder: option.placeholders.first().cloned().unwrap_or_default(),
            default_bool,
            default_int,
            min: option.min,
            max: option.max,
            enum_items,
            default_enum_index,
        });
    }

    Ok(PackDefinition {
        pack_path: simplify_for_display(pack_path.to_path_buf()),
        name: config.config.name.clone(),
        desc: config.config.desc.clone(),
        author: config.config.author.clone(),
        author_url: config.config.author_url.clone(),
        desc_detail: config.config.desc_detail.clone(),
        desc_html: load_pack_desc_html(pack_path, &config.config.desc_file)?,
        tag: config.config.tag.clone(),
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
        dependency_names: dependency_display_names(&config.requirements.pack),
        option_groups: config
            .option_groups
            .iter()
            .map(|group| PackOptionGroupDefinition {
                tag: group.tag.clone(),
                name: if group.name.trim().is_empty() {
                    group.tag.clone()
                } else {
                    group.name.clone()
                },
                desc: group.desc.clone(),
                options: options
                    .iter()
                    .filter(|option| option.tag == group.tag)
                    .cloned()
                    .collect(),
            })
            .collect(),
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
        Ok(config) => {
            let id = normalized_pack_config_id(&config.config.id);
            if id.is_empty() {
                normalized_pack_config_id(&config.config.name)
            } else {
                id
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_option_groups_and_single_level_include() {
        let root = std::env::temp_dir().join(format!("ini-pack-manager-include-test-{}", uuid_like_suffix()));
        let include_dir = root.join("groups");
        fs::create_dir_all(&include_dir).unwrap();
        fs::write(
            root.join("config.toml"),
            r#"
[Config]
Name = "Test"
OptionGroups = ["General", "Advanced"]
Include = ["groups/advanced.toml"]

[General]
Name = "General"
[[General.Options]]
Name = "Enabled"
Type = "bool"
"#,
        )
        .unwrap();
        fs::write(
            include_dir.join("advanced.toml"),
            r#"
[Advanced]
Name = "Advanced"
[[Advanced.Options]]
Name = "Level"
Type = "int"
"#,
        )
        .unwrap();

        let parsed = parse_pack_config(&root).unwrap();
        assert_eq!(parsed.option_groups.len(), 2);
        assert_eq!(parsed.options[0].name, "General.Enabled");
        assert_eq!(parsed.options[1].name, "Advanced.Level");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn wraps_legacy_options_in_general_group() {
        let root = std::env::temp_dir().join(format!("ini-pack-manager-legacy-test-{}", uuid_like_suffix()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("config.toml"),
            r#"
[Config]
Name = "Legacy"

[[Options]]
Name = "Enabled"
Type = "bool"
"#,
        )
        .unwrap();

        let parsed = parse_pack_config(&root).unwrap();
        assert_eq!(parsed.option_groups[0].tag, "General");
        assert_eq!(parsed.option_groups[0].options.len(), 1);
        assert_eq!(parsed.option_groups[0].options[0].tag, "General");
        assert_eq!(parsed.options[0].name, "Enabled");
        let _ = fs::remove_dir_all(root);
    }
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
