const LEGACY_INSTANCE_STORE_RELATIVE_PATH: &str = "config/data/instances.json";
const USER_INSTANCE_STORE_RELATIVE_PATH: &str = "IniPackManager/config/data/instances.json";
const USER_COMPONENT_STATE_RELATIVE_PATH: &str = "IniPackManager/config/data/components.json";
const USER_COMPONENTS_RELATIVE_PATH: &str = "IniPackManager/components";
const USER_REPOSITORY_RELATIVE_PATH: &str = "IniPackManager/repository";
const PROJECT_PRESETS_RELATIVE_PATH: &str = "config/preset";
const PACK_MAIN_FILES: [(&str, &str); 5] = [
    ("Rules", "RulesMain.ini"),
    ("Art", "ArtMain.ini"),
    ("Ai", "AiMain.ini"),
    ("Theme", "ThemeMain.ini"),
    ("Ui", "UIMain.ini"),
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
    #[serde(rename = "Data", default)]
    data: RawPackData,
    #[serde(rename = "Resource", default)]
    _resource: Vec<String>,
    #[serde(rename = "Requirements", default)]
    requirements: RawPackRequirements,
}

#[derive(Debug, Deserialize)]
struct RawPackMeta {
    name: String,
    #[serde(default)]
    desc: String,
    #[serde(default)]
    dir: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    game: String,
    #[serde(default)]
    version: i64,
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
    #[serde(rename = "Files", default)]
    files: Vec<String>,
    #[serde(rename = "Pack", default)]
    pack: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct RawPackDataItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    file: String,
    #[serde(rename = "Options", default)]
    options: Vec<String>,
    #[serde(rename = "need_include", default = "default_need_include")]
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

#[derive(Debug, Deserialize)]
struct RawPackOption {
    name: String,
    #[serde(default)]
    desc: String,
    #[serde(rename = "type")]
    option_type: String,
    #[serde(default, rename = "placeholders")]
    placeholders: Vec<String>,
    #[serde(default)]
    true_result: Option<String>,
    #[serde(default)]
    false_result: Option<String>,
    #[serde(default, deserialize_with = "deserialize_i64_or_single_item_array")]
    min: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_i64_or_single_item_array")]
    max: Option<i64>,
    #[serde(rename = "valueOutputs", default)]
    value_outputs: Vec<String>,
    #[serde(default, rename = "values")]
    values: Vec<String>,
    #[serde(default)]
    results: Vec<String>,
    #[serde(default)]
    default: Option<toml::Value>,
}

#[derive(Debug, Serialize)]
struct PackDefinition {
    pack_path: String,
    name: String,
    desc: String,
    dir: String,
    config_id: String,
    version: i64,
    requirements: PackRequirementDefinition,
    options: Vec<PackOptionDefinition>,
}

#[derive(Debug, Serialize)]
struct PackRequirementDefinition {
    files: Vec<String>,
    pack: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PackOptionDefinition {
    name: String,
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
    config_id: String,
    #[serde(default)]
    version: i64,
    pack_path: String,
    enabled: bool,
    settings: Vec<ComponentSetting>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ComponentStateStore {
    #[serde(default)]
    by_instance: HashMap<String, Vec<ComponentState>>,
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


