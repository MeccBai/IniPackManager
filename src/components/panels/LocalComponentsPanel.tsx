import { Button, Switch, Text } from "@fluentui/react-components";
import type { ComponentState } from "../../types";

type Props = {
  components: ComponentState[];
  selectedInstancePath: string | null;
  activeComponentId: string | null;
  onDelete: (component: ComponentState) => Promise<void>;
  onOpenDetail: (component: ComponentState) => Promise<void>;
  onToggleEnabled: (component: ComponentState, enabled: boolean) => Promise<void>;
  styles: Record<string, string>;
};

export function LocalComponentsPanel(props: Props) {
  const {
    components,
    selectedInstancePath,
    activeComponentId,
    onDelete,
    onOpenDetail,
    onToggleEnabled,
    styles,
  } = props;

  if (components.length === 0) {
    return (
      <div className={styles.optionCard}>
        <Text className={styles.empty}>暂无组件</Text>
      </div>
    );
  }

  return (
    <div className={styles.optionsList}>
      {components.map((component) => (
        <div
          key={component.id}
          className={`${styles.optionCard} ${activeComponentId === component.id ? styles.activeOptionCard : ""}`}
        >
          <Text weight="semibold">{component.name}</Text>
          <Text className={styles.empty}>
            {(component.desc || "无描述") + ` · v${component.version ?? 0}`}
          </Text>
          <div className={styles.itemFooter}>
            <Switch
              checked={component.enabled}
              onChange={(_, data) => void onToggleEnabled(component, data.checked)}
              label={component.enabled ? "已启用" : "已禁用"}
              disabled={!selectedInstancePath}
            />
            <div className={styles.rightAligned}>
              <Button
                size="small"
                appearance="subtle"
                onClick={() => void onDelete(component)}
                disabled={!selectedInstancePath}
              >
                删除组件
              </Button>
              <Button
                size="small"
                appearance="subtle"
                onClick={() => void onOpenDetail(component)}
                disabled={!selectedInstancePath || !component.has_options}
                title={!component.has_options ? "该组件没有可配置选项" : undefined}
              >
                详情设置
              </Button>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
