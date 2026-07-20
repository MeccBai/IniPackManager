const LEGACY_INSTANCE_STORE_RELATIVE_PATH: &str = "config/data/instances.json";
const USER_INSTANCE_STORE_RELATIVE_PATH: &str = "IniPackManager/config/data/instances.json";
const USER_COMPONENT_STATE_RELATIVE_PATH: &str = "IniPackManager/config/data/components.json";
const USER_APP_SETTINGS_RELATIVE_PATH: &str = "IniPackManager/config/data/settings.json";
const USER_REMOTE_INDEX_CACHE_RELATIVE_PATH: &str = "IniPackManager/config/data/remote-index-cache.json";
const PROJECT_PRESETS_RELATIVE_PATH: &str = "config/preset";
const PACK_MAIN_FILES: [(&str, &str); 5] = [
    ("Rules", "RulesMain.ini"),
    ("Art", "ArtMain.ini"),
    ("Ai", "AiMain.ini"),
    ("Theme", "ThemeMain.ini"),
    ("Ui", "UIMain.ini"),
];
const PACK_TAGS: [(&str, &str); 7] = [
    ("General", "常规"),
    ("Base", "基础"),
    ("Tools", "工具"),
    ("Unit", "单位"),
    ("Feature", "功能"),
    ("Modifier", "修改"),
    ("Major", "大型"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstanceRecord {
    #[serde(default)]
    name: String,
    #[serde(default)]
    preset_id: String,
    path: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct InstanceStore {
    #[serde(default)]
    instances: Vec<InstanceRecord>,
}

#[derive(Debug, Serialize)]
struct InstanceMutationResult {
    instance: InstanceRecord,
    instances: Vec<InstanceRecord>,
}

#[derive(Debug, Deserialize)]
struct RawPackConfig {
    #[serde(rename = "Config")]
    config: RawPackMeta,
    #[serde(rename = "Options", default)]
    options: Vec<RawPackOption>,
    #[serde(skip)]
    option_groups: Vec<RawPackOptionGroup>,
    #[serde(rename = "Data", default)]
    data: RawPackData,
    #[serde(rename = "Resource", default)]
    resources: Vec<RawPackResource>,
    #[serde(rename = "Exports", default)]
    exports: HashMap<String, String>,
    #[serde(rename = "Imports", default)]
    imports: HashMap<String, String>,
    #[serde(rename = "Requirements", default)]
    requirements: RawPackRequirements,
}

#[derive(Debug, Deserialize)]
struct RawPackResource {
    #[serde(rename = "File", alias = "file", alias = "Path", alias = "path")]
    file: String,
    #[serde(rename = "Dir", alias = "dir", default)]
    dir: bool,
}

#[derive(Debug, Deserialize)]
struct RawPackMeta {
    #[serde(rename = "Name", alias = "name")]
    name: String,
    #[serde(rename = "Desc", alias = "desc", default)]
    desc: String,
    #[serde(rename = "Author", alias = "author", default)]
    author: String,
    #[serde(rename = "AuthorUrl", alias = "author_url", default)]
    author_url: String,
    #[serde(rename = "DescDetail", alias = "desc_detail", default)]
    desc_detail: String,
    #[serde(rename = "DescFile", alias = "desc_file", default)]
    desc_file: String,
    #[serde(rename = "Tag", alias = "tag", default)]
    tag: String,
    #[serde(rename = "Dir", alias = "dir", default)]
    dir: String,
    #[serde(rename = "Id", alias = "id", default)]
    id: String,
    #[serde(rename = "Game", alias = "game", default)]
    game: String,
    #[serde(rename = "Version", alias = "version", default)]
    version: i64,
    #[serde(
        rename = "OptionGroups",
        alias = "option_groups",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    option_groups: Vec<String>,
    #[serde(
        rename = "Include",
        alias = "include",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    includes: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct RawPackOptionGroup {
    #[serde(rename = "Name", alias = "name", default)]
    name: String,
    #[serde(rename = "Desc", alias = "desc", default)]
    desc: String,
    #[serde(rename = "Options", alias = "options", default)]
    options: Vec<RawPackOption>,
    #[serde(skip)]
    tag: String,
}

#[derive(Debug, Deserialize, Default)]
struct RawPackData {
    #[serde(rename = "Rules", default)]
    rules: Vec<RawPackDataItem>,
    #[serde(rename = "Art", default)]
    art: Vec<RawPackDataItem>,
    #[serde(rename = "Ai", default)]
    ai: Vec<RawPackDataItem>,
    #[serde(rename = "Theme", default)]
    theme: Vec<RawPackDataItem>,
    #[serde(rename = "Ui", default)]
    ui: Vec<RawPackDataItem>,
}

#[derive(Debug, Deserialize, Default)]
struct RawPackRequirements {
    #[serde(rename = "Files", alias = "files", default)]
    files: Vec<String>,
    #[serde(rename = "Pack", alias = "pack", default)]
    pack: Vec<String>,
    #[serde(rename = "MinVersion", alias = "min_version", default)]
    min_version: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct RawPackDataItem {
    #[serde(rename = "Name", alias = "name", default)]
    name: String,
    #[serde(rename = "File", alias = "file", default)]
    file: String,
    #[serde(rename = "Options", alias = "options", default)]
    options: Vec<String>,
    #[serde(
        rename = "NeedInclude",
        alias = "need_include",
        alias = "needInclude",
        default = "default_need_include"
    )]
    need_include: bool,
}

fn default_need_include() -> bool {
    true
}

fn deserialize_i64_or_single_item_array<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<toml::Value>::deserialize(deserializer)?;
    let Some(value) = raw else {
        return Ok(None);
    };
    if let Some(number) = value.as_integer() {
        return Ok(Some(number));
    }
    if let Some(array) = value.as_array() {
        if let Some(number) = array.first().and_then(|item| item.as_integer()) {
            return Ok(Some(number));
        }
    }
    Err(serde::de::Error::custom("需要整数或单元素整数数组"))
}

fn deserialize_string_list_or_single<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<toml::Value>::deserialize(deserializer)?;
    let Some(value) = raw else {
        return Ok(Vec::new());
    };
    if let Some(text) = value.as_str() {
        return Ok(vec![text.to_string()]);
    }
    if let Some(array) = value.as_array() {
        return Ok(array
            .iter()
            .filter_map(|item| item.as_str().map(|text| text.to_string()))
            .collect());
    }
    Err(serde::de::Error::custom("需要字符串或字符串数组"))
}

#[derive(Debug, Deserialize, Clone)]
struct RawPackOption {
    #[serde(rename = "Name", alias = "name")]
    name: String,
    #[serde(rename = "Desc", alias = "desc", default)]
    desc: String,
    #[serde(rename = "UIName", alias = "ui_name", default)]
    ui_name: String,
    #[serde(rename = "Type", alias = "type")]
    option_type: String,
    #[serde(rename = "Control", alias = "control", default)]
    control: bool,
    #[serde(
        rename = "Placeholders",
        alias = "placeholders",
        alias = "Placeholder",
        alias = "placeholder",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    placeholders: Vec<String>,
    #[serde(
        rename = "TrueResult",
        alias = "true_result",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    true_results: Vec<String>,
    #[serde(
        rename = "FalseResult",
        alias = "false_result",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    false_results: Vec<String>,
    #[serde(
        rename = "Min",
        alias = "min",
        default,
        deserialize_with = "deserialize_i64_or_single_item_array"
    )]
    min: Option<i64>,
    #[serde(
        rename = "Max",
        alias = "max",
        default,
        deserialize_with = "deserialize_i64_or_single_item_array"
    )]
    max: Option<i64>,
    #[serde(
        rename = "ValueOutputs",
        alias = "valueOutputs",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    value_outputs: Vec<String>,
    #[serde(
        rename = "Values",
        alias = "values",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    values: Vec<String>,
    #[serde(
        rename = "Results",
        alias = "results",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    results: Vec<String>,
    #[serde(rename = "Default", alias = "default", default)]
    default: Option<toml::Value>,
    #[serde(flatten)]
    extra: HashMap<String, toml::Value>,
    #[serde(skip)]
    tag: String,
}

#[derive(Debug, Serialize)]
struct PackDefinition {
    pack_path: String,
    name: String,
    desc: String,
    author: String,
    author_url: String,
    desc_detail: String,
    desc_html: Option<String>,
    tag: String,
    dir: String,
    config_id: String,
    version: i64,
    requirements: PackRequirementDefinition,
    dependency_names: Vec<String>,
    options: Vec<PackOptionDefinition>,
    option_groups: Vec<PackOptionGroupDefinition>,
}

#[derive(Debug, Serialize)]
struct PackRequirementDefinition {
    files: Vec<String>,
    pack: Vec<String>,
    min_version: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct PackOptionDefinition {
    name: String,
    tag: String,
    ui_name: String,
    desc: String,
    option_type: String,
    placeholder: String,
    default_bool: Option<bool>,
    default_int: Option<i64>,
    min: Option<i64>,
    max: Option<i64>,
    enum_items: Vec<String>,
    default_enum_index: Option<usize>,
}

#[derive(Debug, Serialize)]
struct PackOptionGroupDefinition {
    tag: String,
    name: String,
    desc: String,
    options: Vec<PackOptionDefinition>,
}

#[derive(Debug, Deserialize)]
struct PackSelectionInput {
    name: String,
    value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ComponentSetting {
    name: String,
    value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ComponentState {
    id: String,
    name: String,
    desc: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    config_id: String,
    #[serde(default)]
    version: i64,
    #[serde(default)]
    tag: String,
    pack_path: String,
    enabled: bool,
    #[serde(default)]
    has_options: bool,
    settings: Vec<ComponentSetting>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ComponentStateStore {
    #[serde(default)]
    by_instance: HashMap<String, Vec<ComponentState>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppSettings {
    #[serde(default)]
    registry_url: String,
    #[serde(default)]
    local_repository_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigurationSnapshot {
    schema_version: u32,
    preset_id: String,
    components: Vec<SnapshotComponent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SnapshotComponent {
    config_id: String,
    name: String,
    version: i64,
    enabled: bool,
    settings: Vec<ComponentSetting>,
}

#[derive(Debug, Deserialize)]
struct SaveComponentStateInput {
    instance_path: String,
    component: ComponentState,
    apply: bool,
}

#[derive(Debug, Deserialize)]
struct SetComponentEnabledInput {
    instance_path: String,
    component_id: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct DeleteComponentInput {
    instance_path: String,
    component_id: String,
}

#[derive(Debug, Serialize)]
struct ComponentStateMutationResult {
    components: Vec<ComponentState>,
    message: String,
}

#[derive(Debug, Serialize, Clone)]
struct PresetSummary {
    id: String,
    name: String,
    path: String,
}

#[derive(Debug, Serialize)]
struct AddInstanceConflictCheck {
    has_conflict: bool,
    has_duplicate_instance: bool,
    duplicate_instance_name: Option<String>,
    overwrite_files: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RemoteRegistryIndex {
    #[serde(rename = "SchemaVersion", alias = "schema_version", default)]
    _schema_version: i64,
    #[serde(rename = "PackLists", alias = "PackList", default)]
    pack_lists: Vec<RemoteRegistryPackList>,
}

#[derive(Debug, Deserialize)]
struct RemoteRegistryPackList {
    #[serde(rename = "Game", alias = "game")]
    game: String,
    #[serde(
        rename = "Index",
        alias = "index",
        default,
        deserialize_with = "deserialize_string_list_or_single"
    )]
    index: Vec<String>,
    #[serde(rename = "Name", alias = "name", default)]
    name: String,
    #[serde(rename = "Desc", alias = "desc", default)]
    desc: String,
}

#[derive(Debug, Deserialize, Default)]
struct RemotePackageListFile {
    #[serde(rename = "SchemaVersion", alias = "schema_version", default)]
    _schema_version: i64,
    #[serde(rename = "Packages", alias = "Package", default)]
    packages: Vec<RemotePackageEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RemotePackageEntry {
    #[serde(rename = "Id", alias = "id", default)]
    id: String,
    #[serde(rename = "Name", alias = "name")]
    name: String,
    #[serde(rename = "Author", alias = "author", default)]
    author: String,
    #[serde(rename = "Desc", alias = "desc", default)]
    desc: String,
    #[serde(rename = "Tag", alias = "tag", default)]
    tag: String,
    #[serde(rename = "Version", alias = "version", default)]
    version: i64,
    #[serde(rename = "Url", alias = "url")]
    url: String,
    #[serde(rename = "Sha256", alias = "sha256", default)]
    sha256: String,
    #[serde(rename = "MinVersion", alias = "min_version", default)]
    min_version: String,
}

#[derive(Debug, Serialize)]
struct RemotePackageSummary {
    id: String,
    name: String,
    author: String,
    desc: String,
    tag: String,
    version: i64,
    url: String,
    sha256: String,
    min_version: String,
    incompatible_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct RemotePackageCatalog {
    game: String,
    name: String,
    desc: String,
    packages: Vec<RemotePackageSummary>,
}

#[derive(Debug, Deserialize)]
struct LoadRemotePackagesInput {
    registry_url: String,
    game: String,
    #[serde(default)]
    force_refresh: bool,
}

#[derive(Debug, Deserialize)]
struct ImportRemotePackageInput {
    url: String,
    sha256: Option<String>,
}
