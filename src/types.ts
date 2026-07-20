export type InstanceRecord = {
  name: string;
  preset_id: string;
  path: string;
};

export type PresetSummary = {
  id: string;
  name: string;
  path: string;
};

export type InstanceMutationResult = {
  instance: InstanceRecord;
  instances: InstanceRecord[];
};

export type AddInstanceConflictCheck = {
  has_conflict: boolean;
  has_duplicate_instance: boolean;
  duplicate_instance_name: string | null;
  overwrite_files: string[];
};

export type PackOptionDefinition = {
  name: string;
  tag: string;
  ui_name: string;
  desc: string;
  option_type: string;
  placeholder: string;
  default_bool: boolean | null;
  default_int: number | null;
  min: number | null;
  max: number | null;
  enum_items: string[];
  default_enum_index: number | null;
};

export type PackOptionGroupDefinition = {
  tag: string;
  name: string;
  desc: string;
  options: PackOptionDefinition[];
};

export type PackDefinition = {
  pack_path: string;
  name: string;
  desc: string;
  author: string;
  author_url: string;
  desc_detail: string;
  desc_html: string | null;
  tag: string;
  dir: string;
  config_id: string;
  version: number;
  requirements: {
    files: string[];
    pack: string[];
    min_version?: string | null;
  };
  dependency_names: string[];
  options: PackOptionDefinition[];
  option_groups: PackOptionGroupDefinition[];
};

export type ComponentSetting = {
  name: string;
  value: boolean | number;
};

export type ComponentState = {
  id: string;
  name: string;
  desc: string;
  author: string;
  config_id: string;
  version: number;
  tag: string;
  pack_path: string;
  enabled: boolean;
  has_options: boolean;
  settings: ComponentSetting[];
};

export type ComponentStateMutationResult = {
  components: ComponentState[];
  message: string;
};

export type AppSettings = {
  registry_url: string;
  local_repository_path: string;
  download_concurrency: number;
  download_limit_kib: number;
  http_proxy: string;
  last_read_notice_date: string;
};

export type NoticeItem = { date: string; context: string };

export type NoticeCatalog = {
  enabled: boolean;
  notices: NoticeItem[];
  latest_unread: NoticeItem | null;
};

export type DownloadTask = {
  id: string;
  item: RemotePackageSummary;
  instance_path: string;
  instance_name: string;
  status: "queued" | "downloading" | "completed" | "failed";
  error?: string;
};

export type RemotePackageSummary = {
  id: string;
  name: string;
  author: string;
  desc: string;
  tag: string;
  version: number;
  url: string;
  sha256: string;
  min_version: string;
  local_status: string;
  incompatible_reason: string | null;
};

export type RemotePackageCatalog = {
  game: string;
  name: string;
  desc: string;
  packages: RemotePackageSummary[];
};



