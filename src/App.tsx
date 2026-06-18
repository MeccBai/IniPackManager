import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Button,
  Card,
  CardHeader,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  FluentProvider,
  Input,
  Label,
  Slider,
  Spinner,
  Switch,
  Text,
  Dropdown,
  Option,
  Select,
  webDarkTheme,
  webLightTheme,
} from "@fluentui/react-components";
import { AppsListRegular, BoxMultipleRegular, FolderAddRegular } from "@fluentui/react-icons";
import "@fontsource-variable/open-sans";
import "./App.css";
import { useAppStyles } from "./useAppStyles";
import { GlobalSettingsDialog } from "./components/dialogs/GlobalSettingsDialog";
import { InstanceDetailDialog } from "./components/dialogs/InstanceDetailDialog";
import { LocalComponentsPanel } from "./components/panels/LocalComponentsPanel";
import { RemoteRepositoryPanel } from "./components/panels/RemoteRepositoryPanel";
import type {
  AppSettings,
  AddInstanceConflictCheck,
  ComponentSetting,
  ComponentState,
  ComponentStateMutationResult,
  InstanceMutationResult,
  InstanceRecord,
  PackDefinition,
  PackOptionDefinition,
  PresetSummary,
  RemotePackageCatalog,
  RemotePackageSummary,
} from "./types";

function App() {
  const styles = useAppStyles();
  const [instances, setInstances] = useState<InstanceRecord[]>([]);
  const [presets, setPresets] = useState<PresetSummary[]>([]);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [adding, setAdding] = useState(false);
  const [savingDetail, setSavingDetail] = useState(false);
  const [deletingDetail, setDeletingDetail] = useState(false);
  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [detailDialogOpen, setDetailDialogOpen] = useState(false);
  const [errorDialogOpen, setErrorDialogOpen] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");
  const [overwriteConfirmOpen, setOverwriteConfirmOpen] = useState(false);
  const [overwriteSummary, setOverwriteSummary] = useState("");
  const [globalSettingsOpen, setGlobalSettingsOpen] = useState(false);
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [registryUrl, setRegistryUrl] = useState("");
  const [savingRegistryUrl, setSavingRegistryUrl] = useState(false);
  const [activeCatalogTab, setActiveCatalogTab] = useState<"local" | "remote">("local");
  const [remoteQuery, setRemoteQuery] = useState("");
  const [remoteAuthorFilter, setRemoteAuthorFilter] = useState("");

  const [packLoading, setPackLoading] = useState(false);
  const [packApplying, setPackApplying] = useState(false);
  const [remoteLoading, setRemoteLoading] = useState(false);
  const [remoteImportingUrl, setRemoteImportingUrl] = useState<string | null>(null);
  const [packDetailOpen, setPackDetailOpen] = useState(false);
  const [packDefinition, setPackDefinition] = useState<PackDefinition | null>(null);
  const [components, setComponents] = useState<ComponentState[]>([]);
  const [activeComponentId, setActiveComponentId] = useState<string | null>(null);
  const [editingSettings, setEditingSettings] = useState<Record<string, boolean | number>>({});
  const [remotePackages, setRemotePackages] = useState<RemotePackageSummary[]>([]);
  const [remoteCatalogName, setRemoteCatalogName] = useState("");
  const [remoteCatalogDesc, setRemoteCatalogDesc] = useState("");

  const [newName, setNewName] = useState("");
  const [newPath, setNewPath] = useState("");
  const [newPresetId, setNewPresetId] = useState("");

  const [detailOriginalPath, setDetailOriginalPath] = useState("");
  const [detailName, setDetailName] = useState("");
  const [detailPath, setDetailPath] = useState("");

  const [status, setStatus] = useState("");
  const currentTheme = isDarkMode ? webDarkTheme : webLightTheme;

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", isDarkMode ? "dark" : "light");
  }, [isDarkMode]);

  const openError = (message: string) => {
    setErrorMessage(message);
    setErrorDialogOpen(true);
  };

  const syncSelectedInstance = (nextInstances: InstanceRecord[]) => {
    if (nextInstances.length === 0) {
      setSelectedPath(null);
      return;
    }

    setSelectedPath((current) => {
      if (current && nextInstances.some((item) => item.path === current)) {
        return current;
      }
      return nextInstances[0].path;
    });
  };

  const loadInstances = async () => {
    const data = await invoke<InstanceRecord[]>("list_instances");
    setInstances(data);
    syncSelectedInstance(data);
  };

  const loadPresets = async () => {
    const data = await invoke<PresetSummary[]>("list_presets");
    setPresets(data);
    if (data.length > 0) {
      setNewPresetId((prev) => prev || data[0].id);
    }
  };

  const loadAppSettings = async () => {
    const settings = await invoke<AppSettings>("get_app_settings");
    setRegistryUrl(settings.registry_url ?? "");
  };

  const loadComponents = async (instancePath: string) => {
    const data = await invoke<ComponentState[]>("list_instance_components", { instancePath });
    setComponents(data);
    if (data.length > 0) {
      setActiveComponentId((prev) => prev ?? data[0].id);
    } else {
      setActiveComponentId(null);
    }
  };

  useEffect(() => {
    const init = async () => {
      try {
        await Promise.all([loadInstances(), loadPresets(), loadAppSettings()]);
      } catch (error) {
        setStatus(`初始化失败: ${String(error)}`);
      } finally {
        setLoading(false);
      }
    };

    void init();
  }, []);

  useEffect(() => {
    if (!selectedPath) {
      setComponents([]);
      setActiveComponentId(null);
      setRemotePackages([]);
      setRemoteCatalogName("");
      setRemoteCatalogDesc("");
      return;
    }
    void loadComponents(selectedPath);
  }, [selectedPath]);

  const pickPathForNew = async () => {
    try {
      const picked = await invoke<string | null>("pick_instance_folder");
      if (picked) {
        setNewPath(picked);
      }
    } catch (error) {
      openError(`选择路径失败: ${String(error)}`);
    }
  };

  const pickPathForDetail = async () => {
    try {
      const picked = await invoke<string | null>("pick_instance_folder");
      if (picked) {
        setDetailPath(picked);
      }
    } catch (error) {
      openError(`选择路径失败: ${String(error)}`);
    }
  };

  const addInstance = async () => {
    setAdding(true);
    setStatus("");

    try {
      const preview = await invoke<AddInstanceConflictCheck>("preview_add_instance", {
        name: newName,
        path: newPath,
        presetId: newPresetId,
      });
      if (preview.has_conflict) {
        const lines: string[] = [];
        if (preview.has_duplicate_instance) {
          lines.push(
            `检测到重复实例路径：${
              preview.duplicate_instance_name ? preview.duplicate_instance_name : "（未命名实例）"
            }`,
          );
        }
        if (preview.overwrite_files.length > 0) {
          const showFiles = preview.overwrite_files.slice(0, 8);
          lines.push(`将覆盖 ${preview.overwrite_files.length} 个文件：${showFiles.join("，")}`);
          if (preview.overwrite_files.length > showFiles.length) {
            lines.push("还有更多文件将被覆盖。");
          }
        }
        setOverwriteSummary(lines.join("\n"));
        setOverwriteConfirmOpen(true);
        return;
      }

      await performAddInstance(false);
    } catch (error) {
      openError(String(error));
    } finally {
      setAdding(false);
    }
  };

  const performAddInstance = async (forceOverwrite: boolean) => {
    setAdding(true);
    setStatus("");
    try {
      const result = await invoke<InstanceMutationResult>("add_instance", {
        name: newName,
        path: newPath,
        presetId: newPresetId,
        forceOverwrite,
      });
      setInstances(result.instances);
      syncSelectedInstance(result.instances);
      setSelectedPath(result.instance.path);
      setStatus(`实例已添加: ${result.instance.name}`);
      setAddDialogOpen(false);
      setNewName("");
      setNewPath("");
      setOverwriteConfirmOpen(false);
      setOverwriteSummary("");
    } catch (error) {
      openError(String(error));
    } finally {
      setAdding(false);
    }
  };

  const openDetailDialog = (instance: InstanceRecord) => {
    setSelectedPath(instance.path);
    setDetailOriginalPath(instance.path);
    setDetailName(instance.name);
    setDetailPath(instance.path);
    setDetailDialogOpen(true);
  };

  const saveDetail = async () => {
    setSavingDetail(true);
    setStatus("");

    try {
      const result = await invoke<InstanceMutationResult>("update_instance", {
        originalPath: detailOriginalPath,
        name: detailName,
        path: detailPath,
      });
      setInstances(result.instances);
      setSelectedPath(result.instance.path);
      setDetailOriginalPath(result.instance.path);
      setStatus(`实例已更新: ${result.instance.name}`);
      setDetailDialogOpen(false);
    } catch (error) {
      openError(String(error));
    } finally {
      setSavingDetail(false);
    }
  };

  const deleteCurrentInstance = async () => {
    setDeletingDetail(true);
    setStatus("");

    try {
      const nextInstances = await invoke<InstanceRecord[]>("delete_instance", {
        path: detailOriginalPath,
      });
      setInstances(nextInstances);
      syncSelectedInstance(nextInstances);
      setDetailDialogOpen(false);
      setStatus("实例已删除");
    } catch (error) {
      openError(String(error));
    } finally {
      setDeletingDetail(false);
    }
  };

  const selectedInstance = useMemo(
    () => instances.find((item) => item.path === selectedPath) ?? null,
    [instances, selectedPath],
  );

  const activeComponent = useMemo(
    () => components.find((item) => item.id === activeComponentId) ?? null,
    [components, activeComponentId],
  );
  const topContextText = useMemo(() => {
    const instanceName = selectedInstance?.name?.trim() || "未选择实例";
    const componentName = activeComponent?.name?.trim() || "未选择组件";
    return `${instanceName} - ${componentName}`;
  }, [selectedInstance, activeComponent]);

  const remoteAuthors = useMemo(
    () => [...new Set(remotePackages.map((item) => item.author.trim()).filter(Boolean))].sort((a, b) => a.localeCompare(b)),
    [remotePackages],
  );

  const filteredRemotePackages = useMemo(() => {
    const query = remoteQuery.trim().toLowerCase();
    return remotePackages.filter((item) => {
      if (remoteAuthorFilter && item.author !== remoteAuthorFilter) {
        return false;
      }
      if (!query) {
        return true;
      }
      const haystack = [item.name, item.desc, item.author].join("\n").toLowerCase();
      return haystack.includes(query);
    });
  }, [remoteAuthorFilter, remotePackages, remoteQuery]);

  useEffect(() => {
    if (activeCatalogTab !== "remote" || !selectedInstance) {
      return;
    }
    void refreshRemotePackages();
  }, [activeCatalogTab, registryUrl, selectedInstance]);

  const importPack = async () => {
    if (!selectedInstance) {
      openError("请先在左侧选中一个实例");
      return;
    }

    setPackLoading(true);
    setStatus("");

    try {
      const definition = await invoke<PackDefinition | null>("import_pack_zip");
      if (!definition) {
        setPackLoading(false);
        return;
      }
      await registerImportedPack(definition);
    } catch (error) {
      openError(String(error));
    } finally {
      setPackLoading(false);
    }
  };

  const buildDefaultSettings = (definition: PackDefinition) => {
    const defaults: Record<string, boolean | number> = {};
    const settings: ComponentSetting[] = [];
    for (const option of definition.options) {
      let value: boolean | number = 0;
      if (option.option_type === "bool") {
        value = option.default_bool ?? false;
      } else if (option.option_type === "int") {
        value = option.default_int ?? option.min ?? 0;
      } else if (option.option_type === "enum") {
        value = option.default_enum_index ?? 0;
      }
      defaults[option.name] = value;
      settings.push({ name: option.name, value });
    }
    return { defaults, settings };
  };

  const registerImportedPack = async (definition: PackDefinition) => {
    if (!selectedInstance) {
      openError("请先在左侧选中一个实例");
      return;
    }

    const { defaults, settings } = buildDefaultSettings(definition);
    const existed = components.some(
      (item) =>
        item.config_id &&
        definition.config_id &&
        item.config_id.toLowerCase() === definition.config_id.toLowerCase(),
    );
    const component: ComponentState = {
      id: "",
      name: definition.name,
      desc: definition.desc,
      config_id: definition.config_id || "",
      version: definition.version ?? 0,
      pack_path: definition.pack_path,
      enabled: false,
      settings,
    };

    const result = await invoke<ComponentStateMutationResult>("save_instance_component_state", {
      input: {
        instance_path: selectedInstance.path,
        component,
        apply: false,
      },
    });
    setComponents(result.components);
    const created = result.components[result.components.length - 1] ?? null;
    if (created) {
      setActiveComponentId(created.id);
    }
    setPackDefinition(definition);
    setEditingSettings(defaults);
    setStatus(`${existed ? "已更新" : "已导入"}组件: ${definition.name} v${definition.version ?? 0}`);
  };

  const saveRegistrySettings = async () => {
    setSavingRegistryUrl(true);
    try {
      const settings = await invoke<AppSettings>("save_app_settings_command", {
        settings: { registry_url: registryUrl },
      });
      setRegistryUrl(settings.registry_url ?? "");
      setStatus("云端仓库地址已保存");
    } catch (error) {
      openError(String(error));
    } finally {
      setSavingRegistryUrl(false);
    }
  };

  const refreshRemotePackages = async () => {
    if (!selectedInstance) {
      openError("请先在左侧选中一个实例");
      return;
    }
    setRemoteLoading(true);
    try {
      const data = await invoke<RemotePackageCatalog>("list_remote_packages", {
        input: {
          registry_url: registryUrl,
          game: selectedInstance.preset_id,
        },
      });
      setRemoteCatalogName(data.name ?? "");
      setRemoteCatalogDesc(data.desc ?? "");
      setRemotePackages(data.packages ?? []);
    } catch (error) {
      openError(String(error));
    } finally {
      setRemoteLoading(false);
    }
  };

  const importRemotePackage = async (item: RemotePackageSummary) => {
    setRemoteImportingUrl(item.url);
    try {
      const definition = await invoke<PackDefinition>("import_remote_package", {
        input: {
          url: item.url,
          sha256: item.sha256 || null,
        },
      });
      await registerImportedPack(definition);
      setActiveCatalogTab("local");
    } catch (error) {
      openError(String(error));
    } finally {
      setRemoteImportingUrl(null);
    }
  };

  const deleteComponent = async (component: ComponentState) => {
    if (!selectedInstance) {
      return;
    }
    try {
      const result = await invoke<ComponentStateMutationResult>("delete_instance_component", {
        input: {
          instance_path: selectedInstance.path,
          component_id: component.id,
        },
      });
      setComponents(result.components);
      if (activeComponentId === component.id) {
        setActiveComponentId(result.components[0]?.id ?? null);
      }
      setStatus(result.message);
    } catch (error) {
      openError(String(error));
    }
  };

  const openComponentDetail = async (component: ComponentState) => {
    setActiveComponentId(component.id);
    const definition = await invoke<PackDefinition>("load_pack_definition", {
      packPath: component.pack_path,
    });
    setPackDefinition(definition);

    const currentValues: Record<string, boolean | number> = {};
    for (const option of definition.options) {
      const saved = component.settings.find((item) => item.name === option.name);
      if (saved) {
        currentValues[option.name] = saved.value;
      } else if (option.option_type === "bool") {
        currentValues[option.name] = option.default_bool ?? false;
      } else if (option.option_type === "int") {
        currentValues[option.name] = option.default_int ?? option.min ?? 0;
      } else {
        currentValues[option.name] = option.default_enum_index ?? 0;
      }
    }

    setEditingSettings(currentValues);
    setPackDetailOpen(true);
  };

  const saveComponentDetail = async (applyNow: boolean) => {
    if (!selectedInstance || !activeComponent || !packDefinition) {
      return;
    }
    setPackApplying(true);
    try {
      const settings: ComponentSetting[] = packDefinition.options.map((option) => ({
        name: option.name,
        value: option.option_type === "bool"
          ? Boolean(editingSettings[option.name])
          : Number(editingSettings[option.name] ?? 0),
      }));

      const result = await invoke<ComponentStateMutationResult>("save_instance_component_state", {
        input: {
          instance_path: selectedInstance.path,
          component: {
            ...activeComponent,
            settings,
          },
          apply: applyNow && activeComponent.enabled,
        },
      });

      setComponents(result.components);
      setStatus(result.message);
      setPackDetailOpen(false);
    } catch (error) {
      openError(String(error));
    } finally {
      setPackApplying(false);
    }
  };

  const setComponentEnabled = async (component: ComponentState, enabled: boolean) => {
    if (!selectedInstance) {
      return;
    }
    try {
      const result = await invoke<ComponentStateMutationResult>("set_instance_component_enabled", {
        input: {
          instance_path: selectedInstance.path,
          component_id: component.id,
          enabled,
        },
      });
      setComponents(result.components);
      setStatus(result.message);
    } catch (error) {
      openError(String(error));
    }
  };

  const renderPackOptionEditor = (option: PackOptionDefinition) => {
    const renderType = option.option_type.toLowerCase();
    const value = editingSettings[option.name];

    if (renderType === "bool") {
      return (
        <div key={option.name} className={styles.optionCard}>
          <Text weight="semibold">{option.desc}</Text>
          <Switch
            checked={Boolean(value)}
            onChange={(_, data) =>
              setEditingSettings((prev) => ({ ...prev, [option.name]: data.checked }))
            }
            label={Boolean(value) ? "开启" : "关闭"}
          />
        </div>
      );
    }

    if (renderType === "enum") {
      const enumIndex = Number(value ?? 0);
      const useSlider = option.enum_items.length >= 5;
      return (
        <div key={option.name} className={styles.optionCard}>
          <Text weight="semibold">{option.desc}</Text>
          {useSlider ? (
            <>
              <Slider
                min={0}
                max={Math.max(0, option.enum_items.length - 1)}
                step={1}
                value={enumIndex}
                onChange={(_, data) =>
                  setEditingSettings((prev) => ({ ...prev, [option.name]: data.value }))
                }
              />
              <Text>{option.enum_items[enumIndex] ?? ""}</Text>
            </>
          ) : (
            <Dropdown
              selectedOptions={[String(enumIndex)]}
              value={option.enum_items[enumIndex] ?? ""}
              onOptionSelect={(_, data) => {
                const index = Number(data.optionValue ?? 0);
                setEditingSettings((prev) => ({ ...prev, [option.name]: index }));
              }}
            >
              {option.enum_items.map((item, idx) => (
                <Option key={`${option.name}-${item}-${idx}`} value={String(idx)}>
                  {item}
                </Option>
              ))}
            </Dropdown>
          )}
        </div>
      );
    }

    const intValue = Number(value ?? option.default_int ?? option.min ?? 0);
    return (
      <div key={option.name} className={styles.optionCard}>
        <Text weight="semibold">{option.desc}</Text>
        <Input
          type="number"
          value={String(intValue)}
          onChange={(_, data) =>
            setEditingSettings((prev) => ({
              ...prev,
              [option.name]: Number(data.value || 0),
            }))
          }
        />
      </div>
    );
  };

  return (
    <FluentProvider
      theme={currentTheme}
      className={styles.page}
    >
      <div className={styles.appTopBar}>
        <Text className={styles.contextTitle}>{topContextText}</Text>
        <Button appearance="secondary" onClick={() => setGlobalSettingsOpen(true)}>
          全局设置
        </Button>
      </div>

      <main className={styles.layout}>
        <Card className={`${styles.card} ${styles.sidebarCard}`}>
          <CardHeader
            header={
              <div className={styles.cardHead}>
                <AppsListRegular />
                <Text weight="semibold">实例侧栏</Text>
              </div>
            }
          />

          <div className={styles.sidebarContent}>
            {loading ? (
              <Spinner label="正在加载实例..." />
            ) : instances.length === 0 ? (
              <Text className={styles.empty}>暂无实例</Text>
            ) : (
              <ul className={styles.list}>
                {instances.map((instance, index) => (
                  <li key={instance.path}>
                    <button
                      type="button"
                      className={`${styles.instanceCard} ${
                        selectedPath === instance.path ? styles.instanceCardActive : ""
                      }`}
                      onClick={() => setSelectedPath(instance.path)}
                    >
                      <div className={styles.instanceCardTitle}>
                        <Text weight="semibold" className={styles.instanceNameText}>{instance.name}</Text>
                        <span className={styles.tag}>实例 #{index + 1}</span>
                      </div>
                      <div className={styles.toolbar}>
                        <Text size={200} className={styles.empty}>
                          Preset: {instance.preset_id || "(未设置)"}
                        </Text>
                        <Button
                          size="small"
                          appearance="subtle"
                          onClick={(event) => {
                            event.stopPropagation();
                            openDetailDialog(instance);
                          }}
                        >
                          设置详情
                        </Button>
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className={styles.sidebarBottom}>
            {status.trim().length > 0 && <Text className={styles.status}>{status}</Text>}
            <div className={styles.createButtonRow}>
              <Button
                className={styles.fullWidthButton}
                icon={<FolderAddRegular />}
                appearance="primary"
                onClick={() => setAddDialogOpen(true)}
              >
                新建实例
              </Button>
              {adding && <Spinner size="tiny" label="处理中..." />}
            </div>
          </div>
        </Card>

        <Card className={`${styles.card} ${styles.mainCard}`}>
          <div className={styles.mainTopBar}>
            <div className={styles.cardHead}>
              <BoxMultipleRegular />
              <Text weight="semibold">组件中心</Text>
            </div>
            <div className={styles.rightAligned}>
              {activeCatalogTab === "local" ? (
                <Button
                  appearance="primary"
                  onClick={() => void importPack()}
                  disabled={packLoading}
                >
                  {packLoading ? "导入中..." : "导入组件"}
                </Button>
              ) : (
                <Button
                  appearance="secondary"
                  onClick={() => void refreshRemotePackages()}
                  disabled={remoteLoading || !selectedInstance}
                >
                  {remoteLoading ? "刷新中..." : "刷新仓库"}
                </Button>
              )}
            </div>
          </div>

          <div className={styles.componentPlaceholder}>
            <div className={styles.tabBar}>
              <Button
                className={styles.tabButton}
                appearance={activeCatalogTab === "local" ? "primary" : "secondary"}
                onClick={() => setActiveCatalogTab("local")}
              >
                本地组件
              </Button>
              <Button
                className={styles.tabButton}
                appearance={activeCatalogTab === "remote" ? "primary" : "secondary"}
                onClick={() => setActiveCatalogTab("remote")}
                disabled={!selectedInstance}
              >
                云端仓库
              </Button>
            </div>

            {activeCatalogTab === "local" ? (
              <LocalComponentsPanel
                components={components}
                selectedInstancePath={selectedInstance?.path ?? null}
                activeComponentId={activeComponentId}
                onDelete={deleteComponent}
                onOpenDetail={openComponentDetail}
                onToggleEnabled={setComponentEnabled}
                styles={styles}
              />
            ) : (
              <RemoteRepositoryPanel
                game={selectedInstance?.preset_id ?? ""}
                catalogName={remoteCatalogName}
                catalogDesc={remoteCatalogDesc}
                loading={remoteLoading}
                query={remoteQuery}
                authorFilter={remoteAuthorFilter}
                authors={remoteAuthors}
                packages={filteredRemotePackages}
                importingUrl={remoteImportingUrl}
                onQueryChange={setRemoteQuery}
                onAuthorFilterChange={setRemoteAuthorFilter}
                onRefresh={refreshRemotePackages}
                onImport={importRemotePackage}
                styles={styles}
              />
            )}
          </div>
        </Card>
      </main>

      <GlobalSettingsDialog
        open={globalSettingsOpen}
        onOpenChange={setGlobalSettingsOpen}
        isDarkMode={isDarkMode}
        setIsDarkMode={setIsDarkMode}
        registryUrl={registryUrl}
        setRegistryUrl={setRegistryUrl}
        saveRegistryUrl={saveRegistrySettings}
        savingRegistryUrl={savingRegistryUrl}
        styles={styles}
      />

      <Dialog open={addDialogOpen} onOpenChange={(_, data) => setAddDialogOpen(data.open)}>
        <DialogSurface className={`${styles.dialogSurface} ${styles.addDialogSurface}`}>
          <DialogBody>
            <DialogTitle>新建实例</DialogTitle>
            <DialogContent className={styles.dialogContent}>
              <div className={styles.inlineForm}>
                <Text className={styles.addDialogIntro}>
                  填写实例名称与游戏路径，并选择一个 preset。创建时会自动初始化运行所需文件。
                </Text>

                <div className={styles.addDialogSection}>
                  <div className={styles.fieldGroup}>
                    <Label htmlFor="instance-name">实例名称</Label>
                    <Input
                      id="instance-name"
                      value={newName}
                      onChange={(_, data) => setNewName(data.value)}
                      placeholder="自定义名字"
                    />
                  </div>
                </div>

                <div className={styles.addDialogSection}>
                  <div className={styles.fieldGroup}>
                    <Label htmlFor="instance-path">实例路径</Label>
                    <div className={styles.pathRow}>
                      <Input
                        id="instance-path"
                        value={newPath}
                        onChange={(_, data) => setNewPath(data.value)}
                        placeholder="手动输入路径，或右侧选择文件夹"
                      />
                      <Button onClick={() => void pickPathForNew()}>选择文件夹</Button>
                    </div>
                  </div>
                </div>

                <div className={styles.addDialogSection}>
                  <div className={styles.fieldGroup}>
                    <Label htmlFor="instance-preset">Preset</Label>
                    {presets.length === 0 ? (
                      <Text className={styles.danger}>未找到 preset，请检查项目目录下的 config/preset</Text>
                    ) : (
                      <Select
                        id="instance-preset"
                        value={newPresetId}
                        onChange={(event) => setNewPresetId(event.target.value)}
                      >
                        {!newPresetId && <option value="">请选择 preset</option>}
                        {presets.map((preset) => (
                          <option key={preset.id} value={preset.id}>
                            {preset.name} ({preset.id})
                          </option>
                        ))}
                      </Select>
                    )}
                  </div>
                </div>
              </div>
            </DialogContent>
            <DialogActions>
              <Button appearance="secondary" onClick={() => setAddDialogOpen(false)}>
                取消
              </Button>
              <Button
                appearance="primary"
                onClick={() => void addInstance()}
                disabled={adding || presets.length === 0}
              >
                {adding ? "创建中..." : "创建实例"}
              </Button>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>

      <InstanceDetailDialog
        open={detailDialogOpen}
        onOpenChange={setDetailDialogOpen}
        detailName={detailName}
        setDetailName={setDetailName}
        detailPath={detailPath}
        setDetailPath={setDetailPath}
        pickPathForDetail={pickPathForDetail}
        deleteCurrentInstance={deleteCurrentInstance}
        saveDetail={saveDetail}
        deletingDetail={deletingDetail}
        savingDetail={savingDetail}
        styles={styles}
      />

      <Dialog open={packDetailOpen} onOpenChange={(_, data) => setPackDetailOpen(data.open)}>
        <DialogSurface className={styles.dialogSurface}>
          <DialogBody>
            <DialogTitle>组件详情设置</DialogTitle>
            <DialogContent className={styles.dialogContent}>
              {!packDefinition ? (
                <Text className={styles.empty}>当前没有可配置的组件。</Text>
              ) : (
                <div className={styles.optionsList}>{packDefinition.options.map(renderPackOptionEditor)}</div>
              )}
            </DialogContent>
            <DialogActions>
              <Button appearance="secondary" onClick={() => setPackDetailOpen(false)}>
                关闭
              </Button>
              <Button
                appearance="primary"
                onClick={() => void saveComponentDetail(true)}
                disabled={packApplying || !activeComponent}
              >
                {packApplying ? "保存中..." : "保存并应用"}
              </Button>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>

      <Dialog open={errorDialogOpen} onOpenChange={(_, data) => setErrorDialogOpen(data.open)}>
        <DialogSurface className={styles.dialogSurface}>
          <DialogBody>
            <DialogTitle>操作失败</DialogTitle>
            <DialogContent className={styles.dialogContent}>
              <Text className={styles.danger}>{errorMessage || "发生未知错误"}</Text>
            </DialogContent>
            <DialogActions>
              <Button appearance="primary" onClick={() => setErrorDialogOpen(false)}>
                我知道了
              </Button>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>

      <Dialog open={overwriteConfirmOpen} onOpenChange={(_, data) => setOverwriteConfirmOpen(data.open)}>
        <DialogSurface className={styles.dialogSurface}>
          <DialogBody>
            <DialogTitle>检测到将覆盖已有内容</DialogTitle>
            <DialogContent className={styles.dialogContent}>
              <Text style={{ whiteSpace: "pre-wrap" }}>
                {overwriteSummary || "检测到重复路径或文件覆盖风险，是否继续覆盖？"}
              </Text>
            </DialogContent>
            <DialogActions>
              <Button appearance="secondary" onClick={() => setOverwriteConfirmOpen(false)}>
                取消
              </Button>
              <Button appearance="primary" onClick={() => void performAddInstance(true)} disabled={adding}>
                {adding ? "覆盖中..." : "确认覆盖"}
              </Button>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>

    </FluentProvider>
  );
}

export default App;
