import {
  Button,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  Input,
  Label,
  Switch,
  Text,
} from "@fluentui/react-components";
import { useState } from "react";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  isDarkMode: boolean;
  setIsDarkMode: (checked: boolean) => void;
  registryUrl: string;
  setRegistryUrl: (value: string) => void;
  localRepositoryPath: string;
  setLocalRepositoryPath: (value: string) => void;
  pickLocalRepositoryPath: () => Promise<void>;
  saveRegistryUrl: () => Promise<void>;
  savingRegistryUrl: boolean;
  canManageConfiguration: boolean;
  exportConfiguration: () => Promise<void>;
  importConfiguration: () => Promise<void>;
  styles: Record<string, string>;
};

export function GlobalSettingsDialog(props: Props) {
  const [page, setPage] = useState<"general" | "repository" | "configuration" | "about">("general");
  const { styles } = props;

  return (
    <Dialog open={props.open} onOpenChange={(_, data) => props.onOpenChange(data.open)}>
      <DialogSurface className={styles.settingsDialogSurface}>
        <DialogBody className={styles.settingsDialogBody}>
          <DialogTitle>全局设置</DialogTitle>
          <DialogContent className={styles.settingsDialogContent}>
            <div className={styles.settingsLayout}>
              <nav className={styles.settingsNav} aria-label="设置分类">
                <Button className={styles.settingsNavButton} appearance={page === "general" ? "primary" : "secondary"} onClick={() => setPage("general")}>常规</Button>
                <Button className={styles.settingsNavButton} appearance={page === "repository" ? "primary" : "secondary"} onClick={() => setPage("repository")}>仓库</Button>
                <Button className={styles.settingsNavButton} appearance={page === "configuration" ? "primary" : "secondary"} onClick={() => setPage("configuration")}>配置管理</Button>
                <Button className={styles.settingsNavButton} appearance={page === "about" ? "primary" : "secondary"} onClick={() => setPage("about")}>关于</Button>
              </nav>

              <div className={styles.settingsContent}>
                {page === "general" && (
                  <>
                  <div className={styles.settingsPageHeader}>
                    <Text className={styles.settingsEyebrow}>Appearance</Text>
                    <Text className={styles.settingsTitle}>常规设置</Text>
                    <Text className={styles.settingsLead}>调整应用的显示偏好。</Text>
                  </div>
                  <div className={styles.settingsSection}>
                    <Text weight="semibold">主题设置</Text>
                    <Switch checked={props.isDarkMode} onChange={(_, data) => props.setIsDarkMode(data.checked)} label={props.isDarkMode ? "深色模式" : "浅色模式"} />
                  </div>
                  </>
                )}

                {page === "repository" && (
                  <>
                    <div className={styles.settingsPageHeader}>
                      <Text className={styles.settingsEyebrow}>Repositories</Text>
                      <Text className={styles.settingsTitle}>仓库设置</Text>
                      <Text className={styles.settingsLead}>管理云端索引与本地组件仓库的存储位置。</Text>
                    </div>
                    <div className={styles.settingsSection}>
                      <Text weight="semibold">云端仓库</Text>
                      <div className={styles.fieldGroup}>
                        <Label htmlFor="registry-url">索引地址</Label>
                        <Input id="registry-url" value={props.registryUrl} onChange={(_, data) => props.setRegistryUrl(data.value)} placeholder="输入 index.toml 的公开 URL" />
                        <Text className={styles.fieldHint}>推荐填写公开可访问的 `index.toml` 原始地址。</Text>
                      </div>
                      <Button appearance="secondary" onClick={() => void props.saveRegistryUrl()} disabled={props.savingRegistryUrl}>
                        {props.savingRegistryUrl ? "保存中..." : "保存仓库地址"}
                      </Button>
                    </div>
                    <div className={styles.settingsSection}>
                      <Text weight="semibold">本地仓库</Text>
                      <div className={styles.fieldGroup}>
                        <Label htmlFor="local-repository-path">仓库父目录</Label>
                        <Input id="local-repository-path" value={props.localRepositoryPath} onChange={(_, data) => props.setLocalRepositoryPath(data.value)} placeholder="留空使用默认目录" />
                        <Button appearance="secondary" onClick={() => void props.pickLocalRepositoryPath()}>选择文件夹</Button>
                        <Text className={styles.fieldHint}>实际目录为 &lt;父目录&gt;/components 和 &lt;父目录&gt;/repository。</Text>
                      </div>
                    </div>
                  </>
                )}

                {page === "configuration" && (
                  <>
                  <div className={styles.settingsPageHeader}>
                    <Text className={styles.settingsEyebrow}>Snapshot</Text>
                    <Text className={styles.settingsTitle}>配置管理</Text>
                    <Text className={styles.settingsLead}>将当前实例的 Preset、组件和选项值保存为可迁移快照。</Text>
                  </div>
                  <div className={styles.settingsSection}>
                    <Text weight="semibold">当前实例配置</Text>
                    <Text className={styles.fieldHint}>导出会保存 Preset、组件包和选项值；导入会替换当前实例的组件配置。</Text>
                    <div className={styles.tabBar}>
                      <Button appearance="secondary" disabled={!props.canManageConfiguration} onClick={() => void props.exportConfiguration()}>导出配置</Button>
                      <Button appearance="primary" disabled={!props.canManageConfiguration} onClick={() => void props.importConfiguration()}>导入配置</Button>
                    </div>
                    {!props.canManageConfiguration && <Text className={styles.empty}>请先在主界面选择一个实例。</Text>}
                  </div>
                  </>
                )}

                {page === "about" && (
                  <>
                    <div className={styles.aboutHero}>
                      <Text className={styles.aboutVersion}>VERSION 1.0.0</Text>
                      <Text className={styles.aboutName}>Ini Pack Manager</Text>
                      <Text>为游戏 INI 组件包提供实例、Preset、资源和配置快照管理。</Text>
                    </div>
                    <div className={styles.aboutLinkGrid}>
                      <div className={styles.aboutLinkCard}>
                        <Text weight="semibold">作者</Text>
                        <a className={styles.aboutLink} href="https://github.com/MeccBai/" target="_blank" rel="noreferrer">MeccBai</a>
                      </div>
                      <div className={styles.aboutLinkCard}>
                        <Text weight="semibold">开源仓库</Text>
                        <a className={styles.aboutLink} href="https://github.com/MeccBai/IniPackManager" target="_blank" rel="noreferrer">MeccBai/IniPackManager</a>
                      </div>
                    </div>
                    <div className={styles.settingsSection}>
                      <Text weight="semibold">本地数据结构</Text>
                      <Text className={styles.fieldHint}>配置：%USERPROFILE%\IniPackManager\config\data</Text>
                      <Text className={styles.fieldHint}>组件仓库：&lt;本地仓库&gt;\components</Text>
                      <Text className={styles.fieldHint}>中央仓库：&lt;本地仓库&gt;\repository</Text>
                    </div>
                  </>
                )}
              </div>
            </div>
          </DialogContent>
          <DialogActions>
            <Button appearance="primary" onClick={() => props.onOpenChange(false)}>完成</Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
