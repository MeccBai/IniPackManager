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
    Ok(Some(import_pack_archive(&zip_path)?))
}

fn find_repository_pack(config_id: &str) -> Result<PathBuf, String> {
    let repository = repository_root_dir()?;
    for entry in fs::read_dir(&repository)
        .map_err(|err| format!("读取本地仓库失败 {}: {err}", repository.display()))?
    {
        let path = entry.map_err(|err| format!("读取本地仓库目录项失败: {err}"))?.path();
        if path.is_dir()
            && parse_pack_config(&path)
                .is_ok_and(|config| config.config.id.trim().eq_ignore_ascii_case(config_id))
        {
            return Ok(path);
        }
    }
    Err(format!("本地仓库中未找到组件包: {config_id}"))
}

fn dependent_component_names(
    target: &ComponentState,
    components: &[ComponentState],
) -> Result<Vec<String>, String> {
    let target_config = parse_pack_config(Path::new(target.pack_path.trim()))?;
    let target_ids = [&target_config.config.id, &target_config.config.name]
        .into_iter()
        .map(|value| normalized_pack_config_id(value))
        .filter(|value| !value.is_empty())
        .collect::<HashSet<_>>();
    let mut dependents = Vec::new();

    for component in components.iter().filter(|component| component.enabled && component.id != target.id) {
        let config = parse_pack_config(Path::new(component.pack_path.trim()))?;
        if config
            .requirements
            .pack
            .iter()
            .map(|required| normalized_pack_config_id(required))
            .any(|required| target_ids.contains(&required))
        {
            dependents.push(component.name.clone());
        }
    }
    Ok(dependents)
}

fn ensure_component_can_be_disabled(target: &ComponentState, components: &[ComponentState]) -> Result<(), String> {
    let dependents = dependent_component_names(target, components)?;
    if dependents.is_empty() {
        return Ok(());
    }
    Err(format!(
        "无法关闭组件 {}：以下已启用组件依赖它：{}",
        target.name,
        dependents.join("、")
    ))
}

#[tauri::command]
fn finish_startup(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(splashscreen) = app.get_webview_window("splashscreen") {
        splashscreen
            .close()
            .map_err(|err| format!("关闭启动窗口失败: {err}"))?;
    }
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "未找到主窗口".to_string())?;
    main.show().map_err(|err| format!("显示主窗口失败: {err}"))?;
    main.set_focus().map_err(|err| format!("激活主窗口失败: {err}"))
}

#[tauri::command]
fn launch_instance_game(_app: tauri::AppHandle, instance_path: String) -> Result<(), String> {
    let instance_dir = validate_instance_game_dir(Path::new(instance_path.trim()))?;
    let preset_id = instance_preset_id_for_path(instance_path.trim())?;
    let preset = find_preset_by_id(&preset_id)?;
    let game_relative_path = safe_relative_path(&preset_game_name(&preset)?)
        .map_err(|_| "Game.name 必须是实例目录内的相对路径".to_string())?;
    let game_path = instance_dir.join(game_relative_path);
    if !game_path.is_file() {
        return Err(format!("未找到游戏程序: {}", game_path.display()));
    }

    #[cfg(windows)]
    {
        use std::{ffi::OsStr, os::windows::ffi::OsStrExt, ptr};
        use windows_sys::Win32::{
            UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        };

        fn wide_null(value: &OsStr) -> Vec<u16> {
            value.encode_wide().chain(std::iter::once(0)).collect()
        }

        let operation = wide_null(OsStr::new("open"));
        let executable = wide_null(game_path.as_os_str());
        let working_dir = wide_null(instance_dir.as_os_str());
        let result = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                operation.as_ptr(),
                executable.as_ptr(),
                ptr::null(),
                working_dir.as_ptr(),
                SW_SHOWNORMAL,
            )
        };
        let error_code = result as usize;
        if error_code <= 32 {
            return Err(format!("请求系统启动游戏失败，ShellExecute 错误码: {error_code}"));
        }
        Ok(())
    }

    #[cfg(not(windows))]
    _app.opener()
        .open_path(game_path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|err| format!("请求系统启动游戏失败: {err}"))
}

#[tauri::command]
fn export_instance_configuration(instance_path: String) -> Result<String, String> {
    let instance_dir = validate_instance_game_dir(Path::new(instance_path.trim()))?;
    let preset_id = instance_preset_id_for_path(instance_path.trim())?;
    let store = load_component_state_store(&component_state_store_path()?)?;
    let components = store
        .by_instance
        .get(&instance_key(instance_path.trim()))
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|component| SnapshotComponent {
            config_id: component.config_id,
            name: component.name,
            version: component.version,
            enabled: component.enabled,
            settings: component.settings,
        })
        .collect();
    let snapshot = ConfigurationSnapshot {
        schema_version: 1,
        preset_id,
        components,
    };
    let path = instance_dir.join("IniPackManager.config.json");
    let raw = serde_json::to_string_pretty(&snapshot)
        .map_err(|err| format!("序列化配置快照失败: {err}"))?;
    fs::write(&path, raw).map_err(|err| format!("写入配置快照失败 {}: {err}", path.display()))?;
    Ok(simplify_for_display(path))
}

#[tauri::command]
fn import_instance_configuration(
    app: tauri::AppHandle,
    instance_path: String,
) -> Result<Option<ComponentStateMutationResult>, String> {
    let Some(selected) = app.dialog().file().add_filter("Configuration", &["json"]).blocking_pick_file() else {
        return Ok(None);
    };
    let file = selected.into_path().map_err(|err| format!("无法读取配置文件路径: {err}"))?;
    let raw = fs::read_to_string(&file).map_err(|err| format!("读取配置快照失败 {}: {err}", file.display()))?;
    let snapshot: ConfigurationSnapshot = serde_json::from_str(&raw)
        .map_err(|err| format!("解析配置快照失败 {}: {err}", file.display()))?;
    if snapshot.schema_version != 1 {
        return Err(format!("不支持的配置快照版本: {}", snapshot.schema_version));
    }
    let instance_dir = validate_instance_game_dir(Path::new(instance_path.trim()))?;
    let preset_id = instance_preset_id_for_path(instance_path.trim())?;
    if !snapshot.preset_id.eq_ignore_ascii_case(&preset_id) {
        return Err(format!("Preset 不匹配：快照为 {}，当前实例为 {}", snapshot.preset_id, preset_id));
    }
    let mut components = Vec::new();
    for saved in snapshot.components {
        let pack_path = find_repository_pack(&saved.config_id)?;
        let config = parse_pack_config(&pack_path)?;
        components.push(ComponentState {
            id: component_id(&simplify_for_display(pack_path.clone())),
            name: config.config.name,
            desc: config.config.desc,
            config_id: config.config.id,
            version: config.config.version,
            pack_path: simplify_for_display(pack_path),
            enabled: saved.enabled,
            has_options: !config.options.is_empty(),
            settings: saved.settings,
        });
    }
    let store_path = component_state_store_path()?;
    let mut store = load_component_state_store(&store_path)?;
    let key = instance_key(instance_path.trim());
    let previous = store.by_instance.get(&key).cloned().unwrap_or_default();
    for component in previous.iter().filter(|component| component.enabled) {
        disable_pack_internal(&instance_dir, Path::new(&component.pack_path))?;
    }
    for component in components.iter().filter(|component| component.enabled) {
        let config = parse_pack_config(Path::new(&component.pack_path))?;
        validate_pack_requirements(&instance_dir, &preset_id, &config, &components, Some(&component.id))?;
        let selections: Vec<PackSelectionInput> = component
            .settings
            .iter()
            .map(|item| PackSelectionInput {
                name: item.name.clone(),
                value: item.value.clone(),
            })
            .collect();
        apply_pack_internal(
            &instance_dir,
            Path::new(&component.pack_path),
            &selections,
            &components,
        )?;
    }
    store.by_instance.insert(key, components.clone());
    save_component_state_store(&store_path, &store)?;
    Ok(Some(ComponentStateMutationResult { components, message: "配置快照已导入并应用".to_string() }))
}

#[tauri::command]
fn list_instance_components(instance_path: String) -> Result<Vec<ComponentState>, String> {
    let key = instance_key(instance_path.trim());
    let store_path = component_state_store_path()?;
    let store = load_component_state_store(&store_path)?;
    let mut components = store.by_instance.get(&key).cloned().unwrap_or_default();
    for component in &mut components {
        component.has_options = parse_pack_config(Path::new(component.pack_path.trim()))
            .map(|config| !config.options.is_empty())
            .unwrap_or(false);
    }
    Ok(components)
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
    component.has_options = !pack_config.options.is_empty();
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
        let _ = apply_pack_internal(&instance_dir, &pack_dir, &selections, list)?;
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
        let _ = apply_pack_internal(&instance_dir, &pack_dir, &selections, list)?;
    } else {
        ensure_component_can_be_disabled(&list[component_index], list)?;
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
        ensure_component_can_be_disabled(&component, list)?;
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
    let _ = apply_pack_internal(
        &instance_dir,
        &pack_dir,
        &selections,
        &instance_components,
    )?;

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
