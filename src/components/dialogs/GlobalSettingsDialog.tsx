import {
  Button,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  Switch,
  Text,
} from "@fluentui/react-components";

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  isDarkMode: boolean;
  setIsDarkMode: (checked: boolean) => void;
  styles: Record<string, string>;
};

export function GlobalSettingsDialog(props: Props) {
  const { open, onOpenChange, isDarkMode, setIsDarkMode, styles } = props;
  return (
    <Dialog open={open} onOpenChange={(_, data) => onOpenChange(data.open)}>
      <DialogSurface className={styles.dialogSurface}>
        <DialogBody>
          <DialogTitle>全局设置</DialogTitle>
          <DialogContent className={styles.dialogContent}>
            <div className={styles.modalGrid}>
              <div className={styles.optionCard}>
                <Text weight="semibold">主题设置</Text>
                <Switch
                  checked={isDarkMode}
                  onChange={(_, data) => setIsDarkMode(data.checked)}
                  label={isDarkMode ? "深色模式" : "浅色模式"}
                />
              </div>
              <div className={styles.optionCard}>
                <Text weight="semibold">About</Text>
                <Text className={styles.empty}>Ini Pack Manager</Text>
                <Text className={styles.empty}>版本：0.1.0</Text>
                <Text className={styles.empty}>实例管理、Preset 应用、组件导入、组件启用/禁用与配置持久化。</Text>
                <Text className={styles.empty}>配置：%USERPROFILE%\\IniPackManager\\config\\data</Text>
                <Text className={styles.empty}>组件仓库：%USERPROFILE%\\IniPackManager\\components</Text>
                <Text className={styles.empty}>中央仓库：%USERPROFILE%\\IniPackManager\\repository</Text>
              </div>
            </div>
          </DialogContent>
          <DialogActions>
            <Button appearance="primary" onClick={() => onOpenChange(false)}>
              完成
            </Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}

