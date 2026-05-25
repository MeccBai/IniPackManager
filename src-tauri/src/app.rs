use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    collections::HashMap,
    collections::HashSet,
    fs::File,
    hash::{Hash, Hasher},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};
use tauri_plugin_dialog::DialogExt;
use zip::ZipArchive;

include!("app_types.rs");
include!("app_storage.rs");

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

fn validate_pack_requirements(
    instance_dir: &Path,
    instance_preset_id: &str,
    config: &RawPackConfig,
    all_components: &[ComponentState],
    current_component_id: Option<&str>,
) -> Result<(), String> {
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

struct ValueOutputParser<'a> {
    expr: &'a str,
    bytes: &'a [u8],
    index: usize,
    var_value: f64,
}

impl<'a> ValueOutputParser<'a> {
    fn new(expr: &'a str, var_value: f64) -> Self {
        Self {
            expr,
            bytes: expr.as_bytes(),
            index: 0,
            var_value,
        }
    }

    fn parse(mut self) -> Result<f64, String> {
        let value = self.parse_expression()?;
        self.skip_whitespace();
        if self.index < self.bytes.len() {
            return Err(format!("第 {} 列附近存在非法字符", self.index + 1));
        }
        if !value.is_finite() {
            return Err("计算结果非法（溢出或非数字）".to_string());
        }
        Ok(value)
    }

    fn parse_expression(&mut self) -> Result<f64, String> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            if self.consume(b'+') {
                value += self.parse_term()?;
            } else if self.consume(b'-') {
                value -= self.parse_term()?;
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            if self.consume(b'*') {
                value *= self.parse_factor()?;
            } else if self.consume(b'/') {
                let divisor = self.parse_factor()?;
                if divisor == 0.0 {
                    return Err(format!("第 {} 列出现除以 0", self.index + 1));
                }
                value /= divisor;
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        if self.consume(b'+') {
            return self.parse_factor();
        }
        if self.consume(b'-') {
            return Ok(-self.parse_factor()?);
        }
        if self.consume(b'(') {
            let value = self.parse_expression()?;
            self.skip_whitespace();
            if !self.consume(b')') {
                return Err(format!("第 {} 列缺少右括号", self.index + 1));
            }
            return Ok(value);
        }
        if self.consume_var() {
            return Ok(self.var_value);
        }
        self.parse_number()
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let start = self.index;
        let mut has_digit = false;

        while self.current().is_some_and(|ch| ch.is_ascii_digit()) {
            has_digit = true;
            self.index += 1;
        }

        if self.current() == Some(b'.') {
            self.index += 1;
            while self.current().is_some_and(|ch| ch.is_ascii_digit()) {
                has_digit = true;
                self.index += 1;
            }
        }

        if !has_digit {
            return Err(format!("第 {} 列需要数字或 var", start + 1));
        }

        let raw = &self.expr[start..self.index];
        raw.parse::<f64>()
            .map_err(|_| format!("第 {} 列数字格式错误: {}", start + 1, raw))
    }

    fn consume_var(&mut self) -> bool {
        let start = self.index;
        if self.bytes.len().saturating_sub(start) < 3 {
            return false;
        }
        let head = &self.bytes[start..start + 3];
        if !head.eq_ignore_ascii_case(b"var") {
            return false;
        }
        let next = self.bytes.get(start + 3).copied();
        if next.is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == b'_') {
            return false;
        }
        self.index += 3;
        true
    }

    fn consume(&mut self, expected: u8) -> bool {
        if self.current() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn current(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn skip_whitespace(&mut self) {
        while self.current().is_some_and(|ch| ch.is_ascii_whitespace()) {
            self.index += 1;
        }
    }
}

fn format_value_output_number(value: f64) -> String {
    let rounded = value.round();
    if (value - rounded).abs() < 1e-9
        && rounded >= i64::MIN as f64
        && rounded <= i64::MAX as f64
    {
        return (rounded as i64).to_string();
    }
    let mut text = format!("{value:.10}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn normalize_list_values(raw_list: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in raw_list {
        for part in raw.split(',') {
            let text = part.trim();
            if !text.is_empty() {
                out.push(text.to_string());
            }
        }
    }
    out
}

fn option_placeholders(option: &RawPackOption) -> Vec<String> {
    normalize_list_values(&option.placeholders)
}

fn option_value_outputs(option: &RawPackOption) -> Vec<String> {
    normalize_list_values(&option.value_outputs)
}

fn resolve_option_replacements(
    option: &RawPackOption,
    selection: &serde_json::Value,
) -> Result<Vec<(String, String)>, String> {
    let placeholders = option_placeholders(option);
    if placeholders.is_empty() {
        return Err(format!("选项 {} 缺少 placeholders", option.name));
    }
    let value_outputs = option_value_outputs(option);

    match option.option_type.to_lowercase().as_str() {
        "bool" => {
            if !value_outputs.is_empty() {
                return Err(format!("选项 {} 是 bool，不支持 valueOutputs", option.name));
            }
            let value = selection
                .as_bool()
                .ok_or_else(|| format!("选项 {} 需要 bool 值", option.name))?;
            let replacement = if value {
                option.true_result.clone().unwrap_or_else(|| "true".to_string())
            } else {
                option.false_result.clone().unwrap_or_else(|| "false".to_string())
            };
            Ok(placeholders
                .into_iter()
                .map(|placeholder| (placeholder, replacement.clone()))
                .collect())
        }
        "int" => {
            let value = selection
                .as_i64()
                .or_else(|| selection.as_u64().and_then(|v| i64::try_from(v).ok()))
                .ok_or_else(|| format!("选项 {} 需要 int 值", option.name))?;
            if let Some(min) = option.min {
                if value < min {
                    return Err(format!("选项 {} 不能小于 {}", option.name, min));
                }
            }
            if let Some(max) = option.max {
                if value > max {
                    return Err(format!("选项 {} 不能大于 {}", option.name, max));
                }
            }
            let mut replacements = Vec::new();
            for (index, placeholder) in placeholders.into_iter().enumerate() {
                let replacement = if let Some(expr) = value_outputs.get(index) {
                    let parsed = ValueOutputParser::new(expr, value as f64).parse().map_err(|err| {
                        format!("选项 {} 的 valueOutputs[{}] 无效: {}", option.name, index, err)
                    })?;
                    format_value_output_number(parsed)
                } else {
                    value.to_string()
                };
                replacements.push((placeholder, replacement));
            }
            Ok(replacements)
        }
        "enum" => {
            if !value_outputs.is_empty() {
                return Err(format!("选项 {} 是 enum，不支持 valueOutputs", option.name));
            }
            let index = selection
                .as_i64()
                .or_else(|| selection.as_u64().and_then(|v| i64::try_from(v).ok()))
                .ok_or_else(|| format!("选项 {} 需要 enum 下标值", option.name))?;
            if index < 0 {
                return Err(format!("选项 {} 的 enum 下标非法", option.name));
            }
            let index = index as usize;
            if index >= option.values.len() {
                return Err(format!("选项 {} 的 enum 下标越界", option.name));
            }
            let replacement = if !option.results.is_empty() {
                if index >= option.results.len() {
                    return Err(format!("选项 {} 的 results 配置不足", option.name));
                }
                option.results[index].clone()
            } else {
                option.values[index].clone()
            };
            Ok(placeholders
                .into_iter()
                .map(|placeholder| (placeholder, replacement.clone()))
                .collect())
        }
        other => Err(format!("不支持的选项类型: {}", other)),
    }
}

fn ensure_include_entry(main_ini_path: &Path, include_rel_path: &str) -> Result<(), String> {
    let include_line = format!("+={}", include_rel_path.replace('\\', "/"));
    let include_line_lower = include_line.to_lowercase();

    let mut content = if main_ini_path.exists() {
        fs::read_to_string(main_ini_path)
            .map_err(|err| format!("读取 {} 失败: {err}", main_ini_path.display()))?
    } else {
        String::new()
    };

    if content
        .lines()
        .any(|line| line.trim().to_lowercase() == include_line_lower)
    {
        return Ok(());
    }

    let mut lines: Vec<String> = if content.is_empty() {
        Vec::new()
    } else {
        content.lines().map(|line| line.to_string()).collect()
    };

    let include_section_idx = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("[#include]"));

    match include_section_idx {
        Some(idx) => {
            let insert_idx = lines
                .iter()
                .enumerate()
                .skip(idx + 1)
                .find(|(_, line)| {
                    let trimmed = line.trim();
                    trimmed.starts_with('[') && trimmed.ends_with(']')
                })
                .map(|(line_idx, _)| line_idx)
                .unwrap_or(lines.len());
            lines.insert(insert_idx, include_line);
        }
        None => {
            if !lines.is_empty() && !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            lines.push("[#include]".to_string());
            lines.push(include_line);
        }
    }

    content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    fs::write(main_ini_path, content)
        .map_err(|err| format!("写入 {} 失败: {err}", main_ini_path.display()))
}

fn remove_include_entry(main_ini_path: &Path, include_rel_path: &str) -> Result<(), String> {
    if !main_ini_path.exists() {
        return Ok(());
    }
    let include_line = format!("+={}", include_rel_path.replace('\\', "/")).to_lowercase();
    let content = fs::read_to_string(main_ini_path)
        .map_err(|err| format!("读取 {} 失败: {err}", main_ini_path.display()))?;
    let lines: Vec<String> = content
        .lines()
        .filter(|line| line.trim().to_lowercase() != include_line)
        .map(|line| line.to_string())
        .collect();
    let mut next = lines.join("\n");
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    fs::write(main_ini_path, next)
        .map_err(|err| format!("写入 {} 失败: {err}", main_ini_path.display()))
}

fn instance_key(instance_path: &str) -> String {
    normalize_path_key(instance_path)
}

fn component_id(pack_path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    normalize_path_key(pack_path).hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn safe_relative_path(path_text: &str) -> Result<PathBuf, String> {
    use std::path::Component;
    let normalized = path_text.replace('\\', "/");
    let raw = Path::new(&normalized);
    let mut out = PathBuf::new();

    for component in raw.components() {
        match component {
            Component::Normal(seg) => out.push(seg),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("zip 中存在非法路径: {}", path_text));
            }
        }
    }

    if out.as_os_str().is_empty() {
        return Err(format!("zip 中存在空路径条目: {}", path_text));
    }
    Ok(out)
}

fn extract_zip_to_directory(zip_path: &Path, dest: &Path, strip_prefix: &str) -> Result<(), String> {
    let file = File::open(zip_path)
        .map_err(|err| format!("无法打开 zip 文件 {}: {err}", zip_path.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|err| format!("读取 zip 失败 {}: {err}", zip_path.display()))?;

    fs::create_dir_all(dest).map_err(|err| format!("无法创建组件目录 {}: {err}", dest.display()))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| format!("读取 zip 条目失败: {err}"))?;
        let full_name = entry.name().replace('\\', "/");

        if !strip_prefix.is_empty() && !full_name.starts_with(strip_prefix) {
            continue;
        }

        let rel_name = if strip_prefix.is_empty() {
            full_name.clone()
        } else {
            full_name[strip_prefix.len()..].to_string()
        };

        if rel_name.trim().is_empty() {
            continue;
        }

        let rel_path = safe_relative_path(&rel_name)?;
        let out_path = dest.join(rel_path);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|err| format!("创建目录失败 {}: {err}", out_path.display()))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("创建目录失败 {}: {err}", parent.display()))?;
        }
        let mut out_file = File::create(&out_path)
            .map_err(|err| format!("创建文件失败 {}: {err}", out_path.display()))?;
        io::copy(&mut entry, &mut out_file)
            .map_err(|err| format!("写入文件失败 {}: {err}", out_path.display()))?;
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|err| format!("无法创建目录 {}: {err}", dst.display()))?;

    let entries = fs::read_dir(src).map_err(|err| format!("读取目录失败 {}: {err}", src.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("读取目录项失败 {}: {err}", src.display()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|err| format!("读取文件类型失败 {}: {err}", src_path.display()))?;

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("无法创建目录 {}: {err}", parent.display()))?;
            }
            fs::copy(&src_path, &dst_path).map_err(|err| {
                format!(
                    "复制文件失败 {} -> {}: {err}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

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

fn data_item_file_name(item: &RawPackDataItem) -> Result<String, String> {
    let file = if !item.file.trim().is_empty() {
        item.file.trim().to_string()
    } else {
        item.name.trim().to_string()
    };
    if file.is_empty() {
        return Err("Data 条目缺少 file/name 字段".to_string());
    }
    Ok(file)
}

fn collect_data_items(config: &RawPackConfig) -> [(&str, &Vec<RawPackDataItem>); 5] {
    [
        ("Rules", &config.data.rules),
        ("Art", &config.data.art),
        ("Ai", &config.data.ai),
        ("Theme", &config.data.theme),
        ("Ui", &config.data.ui),
    ]
}

fn apply_pack_internal(
    instance_dir: &Path,
    pack_dir: &Path,
    selections: &[PackSelectionInput],
) -> Result<Vec<String>, String> {
    let pack_config = parse_pack_config(pack_dir)?;
    let option_map: HashMap<String, &RawPackOption> = pack_config
        .options
        .iter()
        .map(|option| (option.name.clone(), option))
        .collect();
    let mut selection_map: HashMap<String, serde_json::Value> = HashMap::new();
    for input in selections {
        selection_map.insert(input.name.clone(), input.value.clone());
    }

    let output_base = pack_output_base(instance_dir, &pack_config.config);
    fs::create_dir_all(&output_base)
        .map_err(|err| format!("无法创建 Pack 输出目录 {}: {err}", output_base.display()))?;
    copy_dir_recursive(pack_dir, &output_base)?;

    let mut generated_rel_paths = Vec::new();
    for (group_name, items) in collect_data_items(&pack_config) {
        if items.is_empty() {
            continue;
        }
        let main_file = PACK_MAIN_FILES
            .iter()
            .find(|(key, _)| *key == group_name)
            .map(|(_, name)| *name)
            .ok_or_else(|| format!("未知 Data 分组: {}", group_name))?;
        let main_ini_path = instance_dir.join(main_file);

        for item in items {
            let data_file_name = data_item_file_name(item)?;
            let output_file = output_base.join(&data_file_name);
            if !output_file.exists() {
                return Err(format!("Pack 数据文件未复制到输出目录: {}", output_file.display()));
            }

            let mut content = fs::read_to_string(&output_file).map_err(|err| {
                format!("读取已复制的数据文件失败 {}: {err}", output_file.display())
            })?;

            for option_name in &item.options {
                let option = option_map
                    .get(option_name)
                    .ok_or_else(|| format!("数据文件 {} 引用了不存在的选项 {}", data_file_name, option_name))?;
                let replacements = if let Some(selected_value) = selection_map.get(option_name) {
                    resolve_option_replacements(option, selected_value)?
                } else {
                    let default_toml = option
                        .default
                        .as_ref()
                        .ok_or_else(|| format!("选项 {} 未提供值且缺少默认值", option_name))?;
                    let default_json = toml_default_to_json(default_toml).ok_or_else(|| {
                        format!("选项 {} 的默认值类型不支持，请使用 bool/int/string 或其单元素数组", option_name)
                    })?;
                    resolve_option_replacements(option, &default_json)?
                };
                for (placeholder, replacement) in replacements {
                    content = content.replace(&placeholder, &replacement);
                }
            }

            if let Some(parent) = output_file.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("无法创建输出目录 {}: {err}", parent.display()))?;
            }
            fs::write(&output_file, content)
                .map_err(|err| format!("写入输出文件失败 {}: {err}", output_file.display()))?;

            let include_rel_path = output_file
                .strip_prefix(instance_dir)
                .map_err(|err| format!("计算 include 路径失败: {err}"))?
                .to_string_lossy()
                .replace('\\', "/");
            if item.need_include {
                ensure_include_entry(&main_ini_path, &include_rel_path)?;
                generated_rel_paths.push(include_rel_path);
            }
        }
    }
    Ok(generated_rel_paths)
}

fn disable_pack_internal(instance_dir: &Path, pack_dir: &Path) -> Result<(), String> {
    let pack_config = parse_pack_config(pack_dir)?;
    let output_base = pack_output_base(instance_dir, &pack_config.config);

    for (group_name, items) in collect_data_items(&pack_config) {
        if items.is_empty() {
            continue;
        }
        let main_file = PACK_MAIN_FILES
            .iter()
            .find(|(key, _)| *key == group_name)
            .map(|(_, name)| *name)
            .ok_or_else(|| format!("未知 Data 分组: {}", group_name))?;
        let main_ini_path = instance_dir.join(main_file);

        for item in items {
            let data_file_name = data_item_file_name(item)?;
            let output_file = output_base.join(&data_file_name);
            let include_rel_path = output_file
                .strip_prefix(instance_dir)
                .map_err(|err| format!("计算 include 路径失败: {err}"))?
                .to_string_lossy()
                .replace('\\', "/");
            if item.need_include {
                remove_include_entry(&main_ini_path, &include_rel_path)?;
            }

            if output_file.exists() {
                let _ = fs::remove_file(&output_file);
            }
        }
    }

    Ok(())
}

fn validate_instance_game_dir(path: &Path) -> Result<PathBuf, String> {
    if !path.exists() {
        return Err(format!("实例路径不存在: {}", path.display()));
    }

    let metadata = fs::metadata(path)
        .map_err(|err| format!("无法读取实例路径信息 {}: {err}", path.display()))?;
    if !metadata.is_dir() {
        return Err(format!("实例路径不是文件夹: {}", path.display()));
    }

    let has_game_exe = path.join("game.exe").is_file();
    let has_gamemd_exe = path.join("gamemd.exe").is_file();
    if !has_game_exe && !has_gamemd_exe {
        return Err("实例目录中未找到 game.exe 或 gamemd.exe".to_string());
    }

    path.canonicalize()
        .map_err(|err| format!("无法解析实例路径 {}: {err}", path.display()))
}

fn ensure_file_exists(path: &Path) -> Result<(), io::Error> {
    if path.exists() {
        return Ok(());
    }
    File::create(path).map(|_| ())
}

fn initialize_instance_runtime_files(dir: &Path) -> Result<(), String> {
    let pack_dir = dir.join("Pack");
    fs::create_dir_all(&pack_dir)
        .map_err(|err| format!("无法创建 Pack 文件夹 {}: {err}", pack_dir.display()))?;

    let ini_files = [
        "AiMain.ini",
        "ArtMain.ini",
        "RulesMain.ini",
        "ThemeMain.ini",
        "UIMain.ini",
    ];

    for file_name in ini_files {
        let file_path = dir.join(file_name);
        ensure_file_exists(&file_path)
            .map_err(|err| format!("无法创建文件 {}: {err}", file_path.display()))?;
    }

    Ok(())
}

#[tauri::command]
fn list_instances() -> Result<Vec<InstanceRecord>, String> {
    let store_path = instance_store_path()?;
    let store = load_instance_store(&store_path)?;
    let mut instances = store.instances;
    for record in &mut instances {
        normalize_record_name(record);
    }
    Ok(instances)
}

#[tauri::command]
fn list_presets() -> Result<Vec<PresetSummary>, String> {
    list_presets_internal()
}

#[tauri::command]
fn pick_instance_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let Some(selected) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };

    let selected_path = selected
        .into_path()
        .map_err(|err| format!("无法读取所选路径: {err}"))?;
    let selected_display = simplify_for_display(selected_path.clone());

    Ok(Some(selected_display))
}

#[tauri::command]
fn pick_pack_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let Some(selected) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };

    let selected_path = selected
        .into_path()
        .map_err(|err| format!("无法读取所选路径: {err}"))?;
    Ok(Some(simplify_for_display(selected_path)))
}

#[tauri::command]
fn load_pack_definition(pack_path: String) -> Result<PackDefinition, String> {
    let pack_path = Path::new(pack_path.trim()).to_path_buf();
    let parsed = parse_pack_config(&pack_path)?;
    build_pack_definition(&pack_path, &parsed)
}

#[tauri::command]
fn import_pack_zip(app: tauri::AppHandle) -> Result<Option<PackDefinition>, String> {
    let Some(selected) = app
        .dialog()
        .file()
        .add_filter("Zip Archive", &["zip"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };

    let zip_path = selected
        .into_path()
        .map_err(|err| format!("无法读取 zip 路径: {err}"))?;

    let (parsed_config, config_prefix) = read_pack_config_from_zip(&zip_path)?;
    let components_root = components_root_dir()?;
    fs::create_dir_all(&components_root).map_err(|err| {
        format!(
            "无法创建组件目录 {}: {err}",
            components_root.display()
        )
    })?;

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
    let mut updated = false;

    let (target_dir, repo_target) = if !normalized_id.is_empty() {
        let mut found_component_dir: Option<PathBuf> = None;
        let mut found_repo_dir: Option<PathBuf> = None;

        if components_root.exists() {
            let entries = fs::read_dir(&components_root)
                .map_err(|err| format!("读取组件目录失败 {}: {err}", components_root.display()))?;
            for entry in entries {
                let entry = entry.map_err(|err| format!("读取组件目录项失败: {err}"))?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let config = parse_pack_config(&path);
                if let Ok(config) = config {
                    if config.config.id.trim().eq_ignore_ascii_case(&normalized_id) {
                        found_component_dir = Some(path.clone());
                        break;
                    }
                }
            }
        }

        if repository_root.exists() {
            let entries = fs::read_dir(&repository_root)
                .map_err(|err| format!("读取中央仓库目录失败 {}: {err}", repository_root.display()))?;
            for entry in entries {
                let entry = entry.map_err(|err| format!("读取中央仓库目录项失败: {err}"))?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let config = parse_pack_config(&path);
                if let Ok(config) = config {
                    if config.config.id.trim().eq_ignore_ascii_case(&normalized_id) {
                        found_repo_dir = Some(path.clone());
                        break;
                    }
                }
            }
        }

        let component_target = found_component_dir.unwrap_or_else(|| unique_component_dir(components_root.join(&base_name)));
        let repo_target = found_repo_dir.unwrap_or_else(|| unique_component_dir(repository_root.join(component_target.file_name().and_then(|n| n.to_str()).unwrap_or("Pack"))));
        if component_target.exists() || repo_target.exists() {
            updated = true;
        }
        (component_target, repo_target)
    } else {
        let target_dir = unique_component_dir(components_root.join(base_name));
        let repo_target = unique_component_dir(
            repository_root.join(
                target_dir
                    .file_name()
                    .and_then(|n| n.to_str())
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
    extract_zip_to_directory(&zip_path, &target_dir, &config_prefix)?;

    if repo_target.exists() {
        fs::remove_dir_all(&repo_target)
            .map_err(|err| format!("覆盖中央仓库目录失败 {}: {err}", repo_target.display()))?;
    }
    copy_dir_recursive(&target_dir, &repo_target)?;

    let definition = build_pack_definition(&repo_target, &parsed_config)?;
    let _ = updated;
    Ok(Some(definition))
}

#[tauri::command]
fn list_instance_components(instance_path: String) -> Result<Vec<ComponentState>, String> {
    let key = instance_key(instance_path.trim());
    let store_path = component_state_store_path()?;
    let store = load_component_state_store(&store_path)?;
    Ok(store.by_instance.get(&key).cloned().unwrap_or_default())
}

#[tauri::command]
fn save_instance_component_state(
    input: SaveComponentStateInput,
) -> Result<ComponentStateMutationResult, String> {
    let instance_dir = validate_instance_game_dir(Path::new(input.instance_path.trim()))?;
    let instance_preset_id = instance_preset_id_for_path(input.instance_path.trim())?;
    initialize_instance_runtime_files(&instance_dir)?;

    let mut component = input.component;
    if component.id.trim().is_empty() {
        component.id = component_id(&component.pack_path);
    }
    component.name = component.name.trim().to_string();
    component.desc = component.desc.trim().to_string();
    if component.name.is_empty() {
        return Err("组件名称不能为空".to_string());
    }
    if component.pack_path.trim().is_empty() {
        return Err("组件路径不能为空".to_string());
    }
    let pack_dir = Path::new(component.pack_path.trim()).to_path_buf();
    let pack_config = parse_pack_config(&pack_dir)?;
    if component.config_id.trim().is_empty() {
        component.config_id = pack_config.config.id.trim().to_string();
    }
    component.version = pack_config.config.version;

    let instance_key = instance_key(&input.instance_path);
    let store_path = component_state_store_path()?;
    let mut store = load_component_state_store(&store_path)?;
    let list = store.by_instance.entry(instance_key).or_default();

    if let Some(existing) = list.iter_mut().find(|item| item.id == component.id) {
        *existing = component.clone();
    } else {
        list.push(component.clone());
    }

    let mut message = "组件配置已保存".to_string();
    if input.apply && component.enabled {
        validate_pack_requirements(
            &instance_dir,
            &instance_preset_id,
            &pack_config,
            list,
            Some(&component.id),
        )?;
        let selections: Vec<PackSelectionInput> = component
            .settings
            .iter()
            .map(|item| PackSelectionInput {
                name: item.name.clone(),
                value: item.value.clone(),
            })
            .collect();
        let _ = apply_pack_internal(&instance_dir, &pack_dir, &selections)?;
        message = "组件已应用并保存".to_string();
    }

    let next_components = list.clone();
    save_component_state_store(&store_path, &store)?;
    Ok(ComponentStateMutationResult {
        components: next_components,
        message,
    })
}

#[tauri::command]
fn set_instance_component_enabled(
    input: SetComponentEnabledInput,
) -> Result<ComponentStateMutationResult, String> {
    let instance_dir = validate_instance_game_dir(Path::new(input.instance_path.trim()))?;
    let instance_preset_id = instance_preset_id_for_path(input.instance_path.trim())?;
    initialize_instance_runtime_files(&instance_dir)?;

    let key = instance_key(&input.instance_path);
    let store_path = component_state_store_path()?;
    let mut store = load_component_state_store(&store_path)?;
    let list = store.by_instance.entry(key).or_default();
    let Some(component_index) = list.iter().position(|item| item.id == input.component_id) else {
        return Err("组件不存在".to_string());
    };
    let component_id_for_check = list[component_index].id.clone();
    let enabled = input.enabled;
    if enabled {
        let pack_dir = Path::new(list[component_index].pack_path.trim()).to_path_buf();
        let pack_config = parse_pack_config(&pack_dir)?;
        validate_pack_requirements(
            &instance_dir,
            &instance_preset_id,
            &pack_config,
            list,
            Some(&component_id_for_check),
        )?;

        let component = &mut list[component_index];
        if component.config_id.trim().is_empty() {
            component.config_id = pack_config.config.id.trim().to_string();
        }
        component.enabled = true;
        let selections: Vec<PackSelectionInput> = component
            .settings
            .iter()
            .map(|item| PackSelectionInput {
                name: item.name.clone(),
                value: item.value.clone(),
            })
            .collect();
        let _ = apply_pack_internal(&instance_dir, &pack_dir, &selections)?;
    } else {
        let component = &mut list[component_index];
        component.enabled = false;
        let pack_dir = Path::new(component.pack_path.trim()).to_path_buf();
        disable_pack_internal(&instance_dir, &pack_dir)?;
    }
    let message = if enabled {
        "组件已启用并应用".to_string()
    } else {
        "组件已禁用并撤销".to_string()
    };

    let next_components = list.clone();
    save_component_state_store(&store_path, &store)?;
    Ok(ComponentStateMutationResult {
        components: next_components,
        message,
    })
}

#[tauri::command]
fn delete_instance_component(
    input: DeleteComponentInput,
) -> Result<ComponentStateMutationResult, String> {
    let instance_dir = validate_instance_game_dir(Path::new(input.instance_path.trim()))?;
    initialize_instance_runtime_files(&instance_dir)?;

    let key = instance_key(&input.instance_path);
    let store_path = component_state_store_path()?;
    let mut store = load_component_state_store(&store_path)?;
    let list = store.by_instance.entry(key).or_default();
    let Some(index) = list.iter().position(|item| item.id == input.component_id) else {
        return Err("组件不存在".to_string());
    };

    let component = list[index].clone();
    if component.enabled {
        let pack_dir = Path::new(component.pack_path.trim()).to_path_buf();
        disable_pack_internal(&instance_dir, &pack_dir)?;
    }
    list.remove(index);

    let next_components = list.clone();
    save_component_state_store(&store_path, &store)?;
    Ok(ComponentStateMutationResult {
        components: next_components,
        message: "组件已删除".to_string(),
    })
}

#[tauri::command]
fn preview_add_instance(name: String, path: String, preset_id: String) -> Result<AddInstanceConflictCheck, String> {
    let normalized_name = normalize_name(&name);
    if normalized_name.is_empty() {
        return Err("实例名称不能为空".to_string());
    }
    let normalized_preset_id = preset_id.trim().to_string();
    if normalized_preset_id.is_empty() {
        return Err("请先选择一个 preset".to_string());
    }

    let normalized_input_path = path.trim().to_string();
    if normalized_input_path.is_empty() {
        return Err("实例路径不能为空".to_string());
    }

    let validated_path = validate_instance_game_dir(Path::new(&normalized_input_path))?;
    let preset = find_preset_by_id(&normalized_preset_id)?;
    let selected_display = simplify_for_display(validated_path.clone());
    let selected_key = normalize_path_key(&selected_display);

    let store_path = instance_store_path()?;
    let store = load_instance_store(&store_path)?;

    let duplicate = store
        .instances
        .iter()
        .find(|instance| normalize_path_key(&instance.path) == selected_key);
    let overwrite_files = detect_preset_overwrite_files(&validated_path, &preset)?;
    let has_duplicate_instance = duplicate.is_some();
    let has_conflict = has_duplicate_instance || !overwrite_files.is_empty();

    Ok(AddInstanceConflictCheck {
        has_conflict,
        has_duplicate_instance,
        duplicate_instance_name: duplicate.map(|item| item.name.clone()),
        overwrite_files,
    })
}

#[tauri::command]
fn add_instance(
    name: String,
    path: String,
    preset_id: String,
    force_overwrite: Option<bool>,
) -> Result<InstanceMutationResult, String> {
    let normalized_name = normalize_name(&name);
    if normalized_name.is_empty() {
        return Err("实例名称不能为空".to_string());
    }
    let normalized_preset_id = preset_id.trim().to_string();
    if normalized_preset_id.is_empty() {
        return Err("请先选择一个 preset".to_string());
    }

    let normalized_input_path = path.trim().to_string();
    if normalized_input_path.is_empty() {
        return Err("实例路径不能为空".to_string());
    }

    let validated_path = validate_instance_game_dir(Path::new(&normalized_input_path))?;
    let preset = find_preset_by_id(&normalized_preset_id)?;
    let selected_display = simplify_for_display(validated_path.clone());
    let selected_key = normalize_path_key(&selected_display);

    let store_path = instance_store_path()?;
    let mut store = load_instance_store(&store_path)?;

    let duplicate_index = store
        .instances
        .iter()
        .position(|instance| normalize_path_key(&instance.path) == selected_key);
    let overwrite_files = detect_preset_overwrite_files(&validated_path, &preset)?;
    let should_overwrite = force_overwrite.unwrap_or(false);
    if (duplicate_index.is_some() || !overwrite_files.is_empty()) && !should_overwrite {
        return Err("检测到重复路径或将覆盖已有文件，请先确认覆盖".to_string());
    }

    let mut added_instance = InstanceRecord {
        name: normalized_name,
        preset_id: normalized_preset_id,
        path: selected_display,
    };
    normalize_record_name(&mut added_instance);

    initialize_instance_runtime_files(&validated_path)?;
    apply_preset_to_instance(&validated_path, &preset)?;

    if let Some(index) = duplicate_index {
        store.instances[index] = added_instance.clone();
    } else {
        store.instances.push(added_instance.clone());
    }
    save_instance_store(&store_path, &store)?;

    Ok(InstanceMutationResult {
        instance: added_instance,
        instances: store.instances,
    })
}

#[tauri::command]
fn update_instance(
    original_path: String,
    name: String,
    path: String,
) -> Result<InstanceMutationResult, String> {
    let original_key = normalize_path_key(&original_path);
    let normalized_name = normalize_name(&name);
    if normalized_name.is_empty() {
        return Err("实例名称不能为空".to_string());
    }

    let normalized_input_path = path.trim().to_string();
    if normalized_input_path.is_empty() {
        return Err("实例路径不能为空".to_string());
    }

    let validated_path = validate_instance_game_dir(Path::new(&normalized_input_path))?;
    initialize_instance_runtime_files(&validated_path)?;
    let next_display = simplify_for_display(validated_path);
    let next_key = normalize_path_key(&next_display);

    let store_path = instance_store_path()?;
    let mut store = load_instance_store(&store_path)?;

    let Some(target_index) = store
        .instances
        .iter()
        .position(|instance| normalize_path_key(&instance.path) == original_key)
    else {
        return Err("要更新的实例不存在".to_string());
    };

    let duplicated = store
        .instances
        .iter()
        .enumerate()
        .any(|(index, instance)| index != target_index && normalize_path_key(&instance.path) == next_key);
    if duplicated {
        return Err("实例路径已存在，请使用其他路径".to_string());
    }

    let mut updated_instance = InstanceRecord {
        name: normalized_name,
        preset_id: store.instances[target_index].preset_id.clone(),
        path: next_display,
    };
    normalize_record_name(&mut updated_instance);
    store.instances[target_index] = updated_instance.clone();
    save_instance_store(&store_path, &store)?;

    Ok(InstanceMutationResult {
        instance: updated_instance,
        instances: store.instances,
    })
}

#[tauri::command]
fn apply_pack(
    instance_path: String,
    pack_path: String,
    selections: Vec<PackSelectionInput>,
) -> Result<String, String> {
    let instance_dir = validate_instance_game_dir(Path::new(instance_path.trim()))?;
    let instance_preset_id = instance_preset_id_for_path(instance_path.trim())?;
    initialize_instance_runtime_files(&instance_dir)?;
    let pack_dir = Path::new(pack_path.trim()).to_path_buf();
    let pack_config = parse_pack_config(&pack_dir)?;
    let store_path = component_state_store_path()?;
    let store = load_component_state_store(&store_path)?;
    let key = instance_key(instance_path.trim());
    let instance_components = store.by_instance.get(&key).cloned().unwrap_or_default();
    validate_pack_requirements(
        &instance_dir,
        &instance_preset_id,
        &pack_config,
        &instance_components,
        None,
    )?;
    let _ = apply_pack_internal(&instance_dir, &pack_dir, &selections)?;

    Ok("Pack 已复制并应用到实例".to_string())
}

#[tauri::command]
fn delete_instance(path: String) -> Result<Vec<InstanceRecord>, String> {
    let key = normalize_path_key(&path);

    let store_path = instance_store_path()?;
    let mut store = load_instance_store(&store_path)?;
    let original_len = store.instances.len();
    store.instances.retain(|instance| normalize_path_key(&instance.path) != key);

    if store.instances.len() == original_len {
        return Err("要删除的实例不存在".to_string());
    }

    save_instance_store(&store_path, &store)?;
    Ok(store.instances)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_instances,
            list_presets,
            list_instance_components,
            preview_add_instance,
            pick_instance_folder,
            pick_pack_folder,
            import_pack_zip,
            load_pack_definition,
            add_instance,
            update_instance,
            delete_instance,
            apply_pack,
            save_instance_component_state,
            set_instance_component_enabled,
            delete_instance_component
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}



