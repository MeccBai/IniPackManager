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
  Text,
} from "@fluentui/react-components";
import { useState } from "react";
import { SettingsPageLayout } from "./SettingsPageLayout";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  detailName: string;
  setDetailName: (v: string) => void;
  detailPath: string;
  setDetailPath: (v: string) => void;
  pickPathForDetail: () => Promise<void>;
  deleteCurrentInstance: () => Promise<void>;
  saveDetail: () => Promise<void>;
  exportConfiguration: () => Promise<void>;
  importConfiguration: () => Promise<void>;
  deletingDetail: boolean;
  savingDetail: boolean;
  styles: Record<string, string>;
};

export function InstanceDetailDialog(props: Props) {
  const [page, setPage] = useState<"details" | "configuration">("details");
  const { styles } = props;

  return (
    <Dialog open={props.open} onOpenChange={(_, data) => props.onOpenChange(data.open)}>
      <DialogSurface className={styles.settingsDialogSurface}>
        <DialogBody className={styles.settingsDialogBody}>
          <DialogTitle>实例详情</DialogTitle>
          <DialogContent className={styles.settingsDialogContent}>
            <SettingsPageLayout
              activePage={page}
              pages={[{ id: "details", label: "实例信息" }, { id: "configuration", label: "配置管理" }]}
              onPageChange={(nextPage) => setPage(nextPage as typeof page)}
              styles={styles}
            >
              {page === "details" && (
                <>
                  <div className={styles.settingsPageHeader}>
                    <Text className={styles.settingsEyebrow}>Instance</Text>
                    <Text className={styles.settingsTitle}>实例信息</Text>
                    <Text className={styles.settingsLead}>修改显示名称或游戏目录，保存后立即生效。</Text>
                  </div>
                  <div className={styles.settingsSection}>
                    <div className={styles.fieldGroup}>
                      <Label htmlFor="detail-name">实例名称</Label>
                      <Input id="detail-name" value={props.detailName} onChange={(_, data) => props.setDetailName(data.value)} placeholder="输入实例显示名称" />
                    </div>
                    <div className={styles.fieldGroup}>
                      <Label htmlFor="detail-path">实例路径</Label>
                      <div className={styles.pathRow}>
                        <Input id="detail-path" value={props.detailPath} onChange={(_, data) => props.setDetailPath(data.value)} placeholder="输入路径，或右侧重新选择" />
                        <Button onClick={() => void props.pickPathForDetail()}>重新选择</Button>
                      </div>
                      <Text className={styles.fieldHint}>路径需包含 `game.exe` 或 `gamemd.exe`</Text>
                    </div>
                  </div>
                  <div className={styles.dangerSection}>
                    <Text weight="semibold" className={styles.danger}>危险操作</Text>
                    <Text className={styles.empty}>删除实例只会移除管理记录，不会删除游戏目录文件。</Text>
                    <Button appearance="secondary" onClick={() => void props.deleteCurrentInstance()} disabled={props.deletingDetail || props.savingDetail}>
                      {props.deletingDetail ? "删除中..." : "删除实例"}
                    </Button>
                  </div>
                </>
              )}
              {page === "configuration" && (
                <>
                  <div className={styles.settingsPageHeader}>
                    <Text className={styles.settingsEyebrow}>Snapshot</Text>
                    <Text className={styles.settingsTitle}>配置管理</Text>
                    <Text className={styles.settingsLead}>配置快照仅作用于当前实例。</Text>
                  </div>
                  <div className={styles.settingsSection}>
                    <Text weight="semibold">导入与导出</Text>
                    <Text className={styles.fieldHint}>导出会保存当前实例的 Preset、组件和选项值；导入会替换并重新应用当前实例的组件配置。</Text>
                    <div className={styles.tabBar}>
                      <Button appearance="secondary" onClick={() => void props.exportConfiguration()}>导出配置</Button>
                      <Button appearance="primary" onClick={() => void props.importConfiguration()}>导入配置</Button>
                    </div>
                  </div>
                </>
              )}
            </SettingsPageLayout>
          </DialogContent>
          <DialogActions>
            <Button appearance="secondary" onClick={() => props.onOpenChange(false)}>关闭</Button>
            <Button appearance="primary" onClick={() => void props.saveDetail()} disabled={props.savingDetail || props.deletingDetail}>
              {props.savingDetail ? "保存中..." : "保存实例信息"}
            </Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
