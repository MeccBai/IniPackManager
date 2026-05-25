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
  deletingDetail: boolean;
  savingDetail: boolean;
  styles: Record<string, string>;
};

export function InstanceDetailDialog(props: Props) {
  const {
    open,
    onOpenChange,
    detailName,
    setDetailName,
    detailPath,
    setDetailPath,
    pickPathForDetail,
    deleteCurrentInstance,
    saveDetail,
    deletingDetail,
    savingDetail,
    styles,
  } = props;

  return (
    <Dialog open={open} onOpenChange={(_, data) => onOpenChange(data.open)}>
      <DialogSurface className={`${styles.dialogSurface} ${styles.detailDialogSurface}`}>
        <DialogBody>
          <DialogTitle>实例设置详情</DialogTitle>
          <DialogContent className={styles.dialogContent}>
            <div className={styles.modalGrid}>
              <div className={styles.detailHeader}>
                <Text className={styles.empty}>修改实例名称或路径，保存后立即生效。</Text>
              </div>

              <div className={styles.addDialogSection}>
                <div className={styles.fieldGroup}>
                  <Label htmlFor="detail-name">实例名称</Label>
                  <Input
                    id="detail-name"
                    value={detailName}
                    onChange={(_, data) => setDetailName(data.value)}
                    placeholder="输入实例显示名称"
                  />
                </div>
              </div>

              <div className={styles.addDialogSection}>
                <div className={styles.fieldGroup}>
                  <Label htmlFor="detail-path">实例路径</Label>
                  <div className={styles.pathRow}>
                    <Input
                      id="detail-path"
                      value={detailPath}
                      onChange={(_, data) => setDetailPath(data.value)}
                      placeholder="输入路径，或右侧重新选择"
                    />
                    <Button onClick={() => void pickPathForDetail()}>重新选择</Button>
                  </div>
                  <Text className={styles.fieldHint}>路径需包含 `game.exe` 或 `gamemd.exe`</Text>
                </div>
              </div>

              <div className={styles.dangerSection}>
                <Text weight="semibold" className={styles.danger}>危险操作</Text>
                <Text className={styles.empty}>删除实例只会移除管理记录，不会删除游戏目录文件。</Text>
                <Button
                  className={styles.detailActionsLeft}
                  appearance="secondary"
                  onClick={() => void deleteCurrentInstance()}
                  disabled={deletingDetail || savingDetail}
                >
                  {deletingDetail ? "删除中..." : "删除实例"}
                </Button>
              </div>
            </div>
          </DialogContent>
          <DialogActions>
            <Button appearance="secondary" onClick={() => onOpenChange(false)}>
              取消
            </Button>
            <Button
              appearance="primary"
              onClick={() => void saveDetail()}
              disabled={savingDetail || deletingDetail}
            >
              {savingDetail ? "保存中..." : "保存修改"}
            </Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}

