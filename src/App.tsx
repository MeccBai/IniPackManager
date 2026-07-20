import { useEffect, useMemo, useRef, useState } from "react";
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
import { SettingsPageLayout } from "./components/dialogs/SettingsPageLayout";
import { LocalComponentsPanel } from "./components/panels/LocalComponentsPanel";
import { RemoteRepositoryPanel } from "./components/panels/RemoteRepositoryPanel";
import { DownloadManagerPanel } from "./components/panels/DownloadManagerPanel";
import { collectCatalogAuthors, collectCatalogTags, filterCatalogItems } from "./catalogFilters";
import { tagLabel } from "./tagTranslations";
import type {
  AppSettings,
  AddInstanceConflictCheck,
  ComponentSetting,
  ComponentState,
  ComponentStateMutationResult,
  DownloadTask,
  InstanceMutationResult,
  InstanceRecord,
  PackDefinition,
  PackOptionDefinition,
  NoticeCatalog,
  NoticeItem,
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
  const [noticeDialogOpen, setNoticeDialogOpen] = useState(false);
  const [noticesEnabled, setNoticesEnabled] = useState(false);
  const [notices, setNotices] = useState<NoticeItem[]>([]);
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [registryUrl, setRegistryUrl] = useState("");
  const [localRepositoryPath, setLocalRepositoryPath] = useState("");
  const [downloadConcurrency, setDownloadConcurrency] = useState(3);
  const [downloadLimitKib, setDownloadLimitKib] = useState(0);
  const [httpProxy, setHttpProxy] = useState("");
  const [savingRegistryUrl, setSavingRegistryUrl] = useState(false);
  const [activeCatalogTab, setActiveCatalogTab] = useState<"local" | "remote" | "downloads">("local");
  const [remoteQuery, setRemoteQuery] = useState("");
  const [remoteAuthorFilter, setRemoteAuthorFilter] = useState("");
  const [localQuery, setLocalQuery] = useState("");
  const [localAuthorFilter, setLocalAuthorFilter] = useState("");
  const [localTagFilter, setLocalTagFilter] = useState("");
  const [remoteTagFilter, setRemoteTagFilter] = useState("");

  const [packLoading, setPackLoading] = useState(false);
  const [packApplying, setPackApplying] = useState(false);
  const [remoteLoading, setRemoteLoading] = useState(false);
  const [downloadTasks, setDownloadTasks] = useState<DownloadTask[]>([]);
  const runningDownloadIds = useRef(new Set<string>());
  const componentRegistrationQueue = useRef(Promise.resolve());
  const [packDetailOpen, setPackDetailOpen] = useState(false);
  const [packDefinition, setPackDefinition] = useState<PackDefinition | null>(null);
  const [activeOptionTag, setActiveOptionTag] = useState("");
  const [activePackPage, setActivePackPage] = useState("overview");
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
    setLocalRepositoryPath(settings.local_repository_path ?? "");
    setDownloadConcurrency(settings.download_concurrency || 3);
    setDownloadLimitKib(settings.download_limit_kib || 0);
    setHttpProxy(settings.http_proxy ?? "");
  };

  const loadNotices = async (openUnread: boolean) => {
    try {
      const catalog = await invoke<NoticeCatalog>("get_notices");
      setNoticesEnabled(catalog.enabled);
      setNotices(catalog.notices);
      if (openUnread && catalog.latest_unread) setNoticeDialogOpen(true);
      return catalog;
    } catch (error) {
      setStatus(`公告获取失败: ${String(error)}`);
      return null;
    }
  };

  const markLatestNoticeRead = async () => {
    const latest = notices[0];
    if (latest) await invoke("mark_notice_read", { date: latest.date });
  };

  const viewNotices = async () => {
    const catalog = await loadNotices(false);
    try {
      if (catalog?.notices[0]) await invoke("mark_notice_read", { date: catalog.notices[0].date });
      setNoticeDialogOpen(true);
    } catch (error) {
      openError(`读取公告失败: ${String(error)}`);
    }
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
        void loadNotices(true);
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

  const launchGame = async (instance: InstanceRecord) => {
    try {
      await invoke("launch_instance_game", { instancePath: instance.path });
      setStatus(`已请求启动游戏: ${instance.name}`);
    } catch (error) {
      openError(String(error));
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
  const activeOptionGroup = useMemo(() => {
    if (!packDefinition) {
      return null;
    }
    return packDefinition.option_groups.find((group) => group.tag === activePackPage)
      ?? packDefinition.option_groups.find((group) => group.tag === activeOptionTag)
      ?? packDefinition.option_groups[0]
      ?? null;
  }, [activeOptionTag, activePackPage, packDefinition]);
  const localAuthors = useMemo(() => collectCatalogAuthors(components), [components]);
  const localTags = useMemo(() => collectCatalogTags(components), [components]);
  const filteredComponents = useMemo(
    () => filterCatalogItems(components, { query: localQuery, author: localAuthorFilter, tag: localTagFilter }),
    [components, localAuthorFilter, localQuery, localTagFilter],
  );

  const remoteAuthors = useMemo(() => collectCatalogAuthors(remotePackages), [remotePackages]);

  const filteredRemotePackages = useMemo(
    () => filterCatalogItems(remotePackages, { query: remoteQuery, author: remoteAuthorFilter, tag: remoteTagFilter }),
    [remoteAuthorFilter, remotePackages, remoteQuery, remoteTagFilter],
  );
  const remoteTags = useMemo(() => collectCatalogTags(remotePackages), [remotePackages]);

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

  const registerImportedPack = async (definition: PackDefinition, target = selectedInstance, openDetail = true) => {
    if (!target) throw new Error("请先在左侧选中一个实例");

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
      author: definition.author,
      config_id: definition.config_id || "",
      version: definition.version ?? 0,
      tag: definition.tag,
      pack_path: definition.pack_path,
      enabled: false,
      has_options: definition.options.length > 0,
      settings,
    };

    const result = await invoke<ComponentStateMutationResult>("save_instance_component_state", {
      input: {
        instance_path: target.path,
        component,
        apply: false,
      },
    });
    if (selectedInstance?.path === target.path) setComponents(result.components);
    const created = result.components[result.components.length - 1] ?? null;
    if (created) {
      setActiveComponentId(created.id);
    }
    if (openDetail) {
      setPackDefinition(definition);
      setActiveOptionTag(definition.option_groups[0]?.tag ?? "");
      setActivePackPage("overview");
      setEditingSettings(defaults);
    }
    setStatus(`${existed ? "已更新" : "已导入"}组件: ${definition.name} v${definition.version ?? 0}`);
  };

  const saveRegistrySettings = async () => {
    setSavingRegistryUrl(true);
    try {
      const settings = await invoke<AppSettings>("save_app_settings_command", {
        settings: {
          registry_url: registryUrl,
          local_repository_path: localRepositoryPath,
          download_concurrency: downloadConcurrency,
          download_limit_kib: downloadLimitKib,
          http_proxy: httpProxy,
        },
      });
      setRegistryUrl(settings.registry_url ?? "");
      setLocalRepositoryPath(settings.local_repository_path ?? "");
      setDownloadConcurrency(settings.download_concurrency || 3);
      setDownloadLimitKib(settings.download_limit_kib || 0);
      setHttpProxy(settings.http_proxy ?? "");
      setStatus("仓库设置已保存");
    } catch (error) {
      openError(String(error));
    } finally {
      setSavingRegistryUrl(false);
    }
  };

  const pickLocalRepositoryPath = async () => {
    try {
      const path = await invoke<string | null>("pick_pack_folder");
      if (path) setLocalRepositoryPath(path);
    } catch (error) {
      openError(`选择本地仓库失败: ${String(error)}`);
    }
  };

  const exportConfiguration = async () => {
    if (!selectedInstance) return;
    try {
      const path = await invoke<string>("export_instance_configuration", { instancePath: selectedInstance.path });
      setStatus(`配置已导出: ${path}`);
    } catch (error) {
      openError(String(error));
    }
  };

  const importConfiguration = async () => {
    if (!selectedInstance || !window.confirm("导入会替换当前实例的组件配置并重新应用启用组件，是否继续？")) return;
    try {
      const result = await invoke<ComponentStateMutationResult | null>("import_instance_configuration", { instancePath: selectedInstance.path });
      if (result) {
        setComponents(result.components);
        setActiveComponentId(result.components[0]?.id ?? null);
        setStatus(result.message);
      }
    } catch (error) {
      openError(String(error));
    }
  };

  const refreshRemotePackages = async (forceRefresh = false) => {
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
          force_refresh: forceRefresh,
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

  const queueRemotePackage = (item: RemotePackageSummary) => {
    if (!selectedInstance) {
      openError("请先在左侧选中一个实例");
      return;
    }
    setDownloadTasks((current) => current.some((task) => task.item.url === item.url && (task.status === "queued" || task.status === "downloading"))
      ? current
      : [...current, { id: crypto.randomUUID(), item, instance_path: selectedInstance.path, instance_name: selectedInstance.name, status: "queued" }]);
    setActiveCatalogTab("downloads");
  };

  const runDownloadTask = async (task: DownloadTask) => {
    if (runningDownloadIds.current.has(task.id)) return;
    runningDownloadIds.current.add(task.id);
    setDownloadTasks((current) => current.map((item) => item.id === task.id ? { ...item, status: "downloading", error: undefined } : item));
    try {
      const definition = await invoke<PackDefinition>("import_remote_package", {
        input: {
          url: task.item.url,
          sha256: task.item.sha256 || null,
        },
      });
      const target = instances.find((instance) => instance.path === task.instance_path);
      const registration = componentRegistrationQueue.current.then(() => registerImportedPack(definition, target, false));
      componentRegistrationQueue.current = registration.catch(() => undefined);
      await registration;
      setDownloadTasks((current) => current.map((item) => item.id === task.id ? { ...item, status: "completed" } : item));
      setStatus(`下载完成并导入：${definition.name}`);
    } catch (error) {
      setDownloadTasks((current) => current.map((item) => item.id === task.id ? { ...item, status: "failed", error: String(error) } : item));
    } finally {
      runningDownloadIds.current.delete(task.id);
    }
  };

  useEffect(() => {
    const available = Math.max(0, downloadConcurrency - runningDownloadIds.current.size);
    downloadTasks.filter((task) => task.status === "queued").slice(0, available).forEach((task) => void runDownloadTask(task));
  }, [downloadConcurrency, downloadTasks]);

  const retryDownloadTask = (taskId: string) => setDownloadTasks((current) => current.map((task) => task.id === taskId ? { ...task, status: "queued", error: undefined } : task));
  const removeDownloadTask = (taskId: string) => setDownloadTasks((current) => current.filter((task) => task.id !== taskId || task.status === "downloading"));

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
    setActiveOptionTag(definition.option_groups[0]?.tag ?? "");
    setActivePackPage("overview");

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
          <Text weight="semibold">{option.ui_name || option.desc}</Text>
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
          <Text weight="semibold">{option.ui_name || option.desc}</Text>
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
      <Text weight="semibold">{option.ui_name || option.desc}</Text>
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
        <Text className={styles.contextTitle}>Ini Pack Manager 1.1.0</Text>
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
                        <Button
                          size="small"
                          appearance="subtle"
                          onClick={(event) => {
                            event.stopPropagation();
                            void launchGame(instance);
                          }}
                        >
                          启动游戏
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
                <Button appearance="primary" onClick={() => void importPack()} disabled={packLoading}>{packLoading ? "导入中..." : "导入组件"}</Button>
              ) : (
                <Button
                  appearance="secondary"
                  onClick={() => void refreshRemotePackages(true)}
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
              <Button
                className={styles.tabButton}
                appearance={activeCatalogTab === "downloads" ? "primary" : "secondary"}
                onClick={() => setActiveCatalogTab("downloads")}
              >
                下载管理{downloadTasks.filter((task) => task.status === "queued" || task.status === "downloading").length ? ` (${downloadTasks.filter((task) => task.status === "queued" || task.status === "downloading").length})` : ""}
              </Button>
            </div>

            {activeCatalogTab === "local" ? (
              <LocalComponentsPanel
                components={filteredComponents}
                selectedInstancePath={selectedInstance?.path ?? null}
                activeComponentId={activeComponentId}
                onDelete={deleteComponent}
                onOpenDetail={openComponentDetail}
                onToggleEnabled={setComponentEnabled}
                tagFilter={localTagFilter}
                tags={localTags}
                query={localQuery}
                authorFilter={localAuthorFilter}
                authors={localAuthors}
                onQueryChange={setLocalQuery}
                onAuthorFilterChange={setLocalAuthorFilter}
                onTagFilterChange={setLocalTagFilter}
                styles={styles}
              />
            ) : activeCatalogTab === "remote" ? (
              <RemoteRepositoryPanel
                game={selectedInstance?.preset_id ?? ""}
                catalogName={remoteCatalogName}
                catalogDesc={remoteCatalogDesc}
                loading={remoteLoading}
                query={remoteQuery}
                authorFilter={remoteAuthorFilter}
                authors={remoteAuthors}
                tagFilter={remoteTagFilter}
                tags={remoteTags}
                onTagFilterChange={setRemoteTagFilter}
                packages={filteredRemotePackages}
                downloadingUrls={downloadTasks.filter((task) => task.status === "queued" || task.status === "downloading").map((task) => task.item.url)}
                onQueryChange={setRemoteQuery}
                onAuthorFilterChange={setRemoteAuthorFilter}
                onRefresh={() => refreshRemotePackages(true)}
                onImport={queueRemotePackage}
                styles={styles}
              />
            ) : (
              <DownloadManagerPanel tasks={downloadTasks} onRetry={retryDownloadTask} onRemove={removeDownloadTask} styles={styles} />
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
        localRepositoryPath={localRepositoryPath}
        setLocalRepositoryPath={setLocalRepositoryPath}
        downloadConcurrency={downloadConcurrency}
        setDownloadConcurrency={setDownloadConcurrency}
        downloadLimitKib={downloadLimitKib}
        setDownloadLimitKib={setDownloadLimitKib}
        httpProxy={httpProxy}
        setHttpProxy={setHttpProxy}
        pickLocalRepositoryPath={pickLocalRepositoryPath}
        saveRegistryUrl={saveRegistrySettings}
        savingRegistryUrl={savingRegistryUrl}
        noticesEnabled={noticesEnabled}
        onViewNotices={() => void viewNotices()}
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
        exportConfiguration={exportConfiguration}
        importConfiguration={importConfiguration}
        deletingDetail={deletingDetail}
        savingDetail={savingDetail}
        styles={styles}
      />

      <Dialog open={packDetailOpen} onOpenChange={(_, data) => setPackDetailOpen(data.open)}>
        <DialogSurface className={styles.settingsDialogSurface}>
          <DialogBody className={styles.settingsDialogBody}>
            <DialogTitle>组件详情设置</DialogTitle>
            <DialogContent className={styles.settingsDialogContent}>
              {!packDefinition ? (
                <Text className={styles.empty}>当前没有可配置的组件。</Text>
              ) : (
                <SettingsPageLayout
                  activePage={activePackPage}
                  pages={[{ id: "overview", label: "概览" }, ...packDefinition.option_groups.map((group) => ({ id: group.tag, label: group.name }))]}
                  onPageChange={(page) => {
                    setActivePackPage(page);
                    if (page !== "overview") setActiveOptionTag(page);
                  }}
                  styles={styles}
                >
                    {activePackPage === "overview" ? (
                      <>
                        <div className={styles.settingsPageHeader}>
                          <Text className={styles.settingsEyebrow}>TAG · {tagLabel(packDefinition.tag)}</Text>
                          <div className={styles.packTitleRow}>
                            <Text className={styles.settingsTitle}>{packDefinition.name}</Text>
                            {packDefinition.author_url.trim() ? (
                              <a className={styles.packAuthorLink} href={packDefinition.author_url} target="_blank" rel="noreferrer">
                                {packDefinition.author.trim() || "未知作者"}
                              </a>
                            ) : (
                              <Text className={styles.packAuthorText}>{packDefinition.author.trim() || "未知作者"}</Text>
                            )}
                          </div>
                          <Text className={styles.settingsLead}>{packDefinition.desc || "暂无简介"}</Text>
                        </div>
                        <div className={styles.settingsSection}>
                          <Text weight="semibold">详细说明</Text>
                          <Text>{packDefinition.desc_detail || packDefinition.desc || "暂无详细说明。"}</Text>
                          {packDefinition.desc_html && <iframe className={styles.packDescriptionHtml} sandbox="" srcDoc={packDefinition.desc_html} title={`${packDefinition.name} 详细说明`} />}
                        </div>
                        <div className={styles.settingsSection}>
                          <Text weight="semibold">依赖组件</Text>
                          {packDefinition.dependency_names.length === 0 ? <Text className={styles.empty}>此组件不依赖其他组件。</Text> : packDefinition.dependency_names.map((name) => <Text key={name}>{name}</Text>)}
                        </div>
                      </>
                    ) : activeOptionGroup && (
                      <>
                        <div className={styles.settingsPageHeader}>
                          <Text className={styles.settingsEyebrow}>TAG · {activeOptionGroup.tag}</Text>
                          <Text className={styles.settingsTitle}>{activeOptionGroup.name}</Text>
                          {activeOptionGroup.desc && <Text className={styles.settingsLead}>{activeOptionGroup.desc}</Text>}
                        </div>
                        <div className={styles.optionsList}>
                          {activeOptionGroup.options.map(renderPackOptionEditor)}
                        </div>
                      </>
                    )}
                </SettingsPageLayout>
              )}
            </DialogContent>
            <DialogActions>
              <Button appearance="secondary" onClick={() => setPackDetailOpen(false)}>
                关闭
              </Button>
              <Button
                appearance="primary"
                onClick={() => void saveComponentDetail(true)}
                disabled={packApplying || !activeComponent || packDefinition?.options.length === 0}
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

      <Dialog open={noticeDialogOpen} onOpenChange={(_, data) => {
        setNoticeDialogOpen(data.open);
        if (!data.open) void markLatestNoticeRead();
      }}>
        <DialogSurface className={styles.dialogSurface}>
          <DialogBody>
            <DialogTitle>公告</DialogTitle>
            <DialogContent className={styles.optionsList}>
              {notices.map((notice) => (
                <div key={`${notice.date}-${notice.context}`} className={styles.optionCard}>
                  <Text weight="semibold">{notice.date}</Text>
                  <Text style={{ whiteSpace: "pre-wrap" }}>{notice.context}</Text>
                </div>
              ))}
            </DialogContent>
            <DialogActions>
              <Button appearance="primary" onClick={() => { setNoticeDialogOpen(false); void markLatestNoticeRead(); }}>我知道了</Button>
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
