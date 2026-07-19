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

fn resource_file_path(resource: &RawPackResource) -> Result<PathBuf, String> {
    let file = resource.file.trim();
    if file.is_empty() {
        return Err("Resource 条目缺少 File 字段".to_string());
    }
    safe_relative_path(file).map_err(|_| format!("Resource 文件路径非法: {file}"))
}

fn copy_pack_resources(
    instance_dir: &Path,
    pack_dir: &Path,
    output_base: &Path,
    resources: &[RawPackResource],
) -> Result<(), String> {
    for resource in resources {
        let relative_path = resource_file_path(resource)?;
        let source = pack_dir.join(&relative_path);
        if !source.is_file() {
            return Err(format!("Resource 文件不存在: {}", source.display()));
        }

        let target_base = if resource.dir { output_base } else { instance_dir };
        let target = target_base.join(&relative_path);
        if !resource.dir {
            let packed_copy = output_base.join(&relative_path);
            if packed_copy.exists() {
                fs::remove_file(&packed_copy).map_err(|err| {
                    format!("移除 Pack 中的 Resource 文件失败 {}: {err}", packed_copy.display())
                })?;
            }
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("无法创建资源目录 {}: {err}", parent.display()))?;
        }
        fs::copy(&source, &target).map_err(|err| {
            format!("复制 Resource 文件失败 {} -> {}: {err}", source.display(), target.display())
        })?;
    }
    Ok(())
}

fn remove_pack_resources(
    instance_dir: &Path,
    output_base: &Path,
    resources: &[RawPackResource],
) -> Result<(), String> {
    for resource in resources {
        let relative_path = resource_file_path(resource)?;
        let target_base = if resource.dir { output_base } else { instance_dir };
        let target = target_base.join(relative_path);
        if target.exists() {
            fs::remove_file(&target)
                .map_err(|err| format!("删除 Resource 文件失败 {}: {err}", target.display()))?;
        }
    }
    Ok(())
}

fn apply_default_placeholders(content: String, pack_meta: &RawPackMeta) -> String {
    content
        .replace("{Dir}", &pack_dir_placeholder(pack_meta))
        .replace("{Id}", pack_meta.id.trim())
        .replace("{Name}", pack_meta.name.trim())
}

fn resolve_pack_exports(config: &RawPackConfig) -> HashMap<String, String> {
    config
        .exports
        .iter()
        .map(|(name, value)| (name.clone(), apply_default_placeholders(value.clone(), &config.config)))
        .collect()
}

fn resolve_import_placeholders(
    current_pack_dir: &Path,
    components: &[ComponentState],
    imports: &HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    if imports.is_empty() {
        return Ok(HashMap::new());
    }

    let mut resolved = HashMap::new();

    for (alias, reference) in imports {
        let Some((pack_name, export_name)) = reference.trim().split_once('.') else {
            return Err(format!("Imports.{} 必须使用 包标识.导出名 格式", alias));
        };
        if alias.trim().is_empty() || pack_name.trim().is_empty() || export_name.trim().is_empty() {
            return Err(format!("Imports.{} 配置无效", alias));
        }

        let mut value = None;
        for component in components.iter().filter(|component| component.enabled) {
            let pack_dir = Path::new(component.pack_path.trim());
            if normalize_path_key(&simplify_for_display(pack_dir.to_path_buf()))
                == normalize_path_key(&simplify_for_display(current_pack_dir.to_path_buf()))
            {
                continue;
            }
            let config = parse_pack_config(pack_dir)?;
            let matches_pack = config.config.id.trim().eq_ignore_ascii_case(pack_name.trim())
                || config.config.name.trim().eq_ignore_ascii_case(pack_name.trim());
            if !matches_pack {
                continue;
            }
            let exports = resolve_pack_exports(&config);
            value = exports
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(export_name.trim()))
                .map(|(_, value)| value.clone());
            if value.is_none() {
                return Err(format!("组件 {} 未导出 {}", pack_name, export_name));
            }
            break;
        }
        let value = value.ok_or_else(|| format!("未找到已启用的导入组件 {}", pack_name))?;
        resolved.insert(alias.trim().to_string(), value);
    }
    Ok(resolved)
}

fn apply_import_placeholders(content: String, imports: &HashMap<String, String>) -> String {
    imports.iter().fold(content, |content, (name, value)| {
        content.replace(&format!("{{{}}}", name), value)
    })
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
    components: &[ComponentState],
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
    copy_pack_resources(instance_dir, pack_dir, &output_base, &pack_config.resources)?;
    let import_placeholders =
        resolve_import_placeholders(pack_dir, components, &pack_config.imports)?;

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
            content = apply_control_blocks(
                content,
                &option_map,
                &selection_map,
                &data_file_name,
            )?;
            content = apply_default_placeholders(content, &pack_config.config);
            content = apply_import_placeholders(content, &import_placeholders);

            for option_name in &item.options {
                let option = option_map
                    .get(option_name)
                    .ok_or_else(|| format!("数据文件 {} 引用了不存在的选项 {}", data_file_name, option_name))?;
                if option.control {
                    continue;
                }
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

    remove_pack_resources(instance_dir, &output_base, &pack_config.resources)?;

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
