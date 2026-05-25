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

export type PackDefinition = {
  pack_path: string;
  name: string;
  desc: string;
  dir: string;
  config_id: string;
  version: number;
  requirements: {
    files: string[];
    pack: string[];
  };
  options: PackOptionDefinition[];
};

export type ComponentSetting = {
  name: string;
  value: boolean | number;
};

export type ComponentState = {
  id: string;
  name: string;
  desc: string;
  config_id: string;
  version: number;
  pack_path: string;
  enabled: boolean;
  settings: ComponentSetting[];
};

export type ComponentStateMutationResult = {
  components: ComponentState[];
  message: string;
};



