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
import { SettingsPageLayout } from "./SettingsPageLayout";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  isDarkMode: boolean;
  setIsDarkMode: (checked: boolean) => void;
  registryUrl: string;
  setRegistryUrl: (value: string) => void;
  localRepositoryPath: string;
  setLocalRepositoryPath: (value: string) => void;
  downloadConcurrency: number;
  setDownloadConcurrency: (value: number) => void;
  downloadLimitKib: number;
  setDownloadLimitKib: (value: number) => void;
  httpProxy: string;
  setHttpProxy: (value: string) => void;
  pickLocalRepositoryPath: () => Promise<void>;
  saveRegistryUrl: () => Promise<void>;
  savingRegistryUrl: boolean;
  noticesEnabled: boolean;
  onViewNotices: () => void;
  styles: Record<string, string>;
};

export function GlobalSettingsDialog(props: Props) {
  const [page, setPage] = useState<"general" | "repository" | "download" | "about">("general");
  const { styles } = props;

  return (
    <Dialog open={props.open} onOpenChange={(_, data) => props.onOpenChange(data.open)}>
      <DialogSurface className={styles.settingsDialogSurface}>
        <DialogBody className={styles.settingsDialogBody}>
          <DialogTitle>全局设置</DialogTitle>
          <DialogContent className={styles.settingsDialogContent}>
            <SettingsPageLayout
              activePage={page}
              pages={[{ id: "general", label: "常规" }, { id: "repository", label: "仓库" }, { id: "download", label: "下载" }, { id: "about", label: "关于" }]}
              onPageChange={(nextPage) => setPage(nextPage as typeof page)}
              styles={styles}
            >
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

                {page === "download" && (
                  <>
                    <div className={styles.settingsPageHeader}>
                      <Text className={styles.settingsEyebrow}>Downloads</Text>
                      <Text className={styles.settingsTitle}>下载管理</Text>
                      <Text className={styles.settingsLead}>设置云端索引和组件包下载的网络策略。</Text>
                    </div>
                    <div className={styles.settingsSection}>
                      <Text weight="semibold">并发与限速</Text>
                      <div className={styles.fieldGroup}>
                        <Label htmlFor="download-concurrency">最大并行任务数</Label>
                        <Input id="download-concurrency" type="number" min={1} max={8} value={String(props.downloadConcurrency)} onChange={(_, data) => props.setDownloadConcurrency(Math.max(1, Number(data.value) || 1))} />
                        <Text className={styles.fieldHint}>范围为 1 至 8；多个任务会按此数量同时执行。</Text>
                      </div>
                      <div className={styles.fieldGroup}>
                        <Label htmlFor="download-limit">单任务限速（KiB/s）</Label>
                        <Input id="download-limit" type="number" min={0} value={String(props.downloadLimitKib)} onChange={(_, data) => props.setDownloadLimitKib(Math.max(0, Number(data.value) || 0))} />
                        <Text className={styles.fieldHint}>填写 0 表示不限速。限速对每个并行下载任务分别生效。</Text>
                      </div>
                    </div>
                    <div className={styles.settingsSection}>
                      <Text weight="semibold">HTTP 代理</Text>
                      <div className={styles.fieldGroup}>
                        <Label htmlFor="http-proxy">代理地址</Label>
                        <Input id="http-proxy" value={props.httpProxy} onChange={(_, data) => props.setHttpProxy(data.value)} placeholder="http://127.0.0.1:7890" />
                        <Text className={styles.fieldHint}>留空直连。索引刷新和组件包下载都会通过该 HTTP 代理访问。</Text>
                      </div>
                      <Button appearance="secondary" onClick={() => void props.saveRegistryUrl()} disabled={props.savingRegistryUrl}>{props.savingRegistryUrl ? "保存中..." : "保存下载设置"}</Button>
                    </div>
                  </>
                )}

                {page === "about" && (
                  <>
                    <div className={styles.aboutHero}>
                      <Text className={styles.aboutVersion}>VERSION 1.1.0</Text>
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
                      {props.noticesEnabled && <Button appearance="secondary" onClick={props.onViewNotices}>查看公告</Button>}
                    </div>
                  </>
                )}
            </SettingsPageLayout>
          </DialogContent>
          <DialogActions>
            <Button appearance="primary" onClick={() => props.onOpenChange(false)}>完成</Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
