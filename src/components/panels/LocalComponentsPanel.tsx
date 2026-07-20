import { Button, Switch, Text } from "@fluentui/react-components";
import type { ComponentState } from "../../types";
import { tagLabel } from "../../tagTranslations";
import { RepositoryFilters } from "./RepositoryFilters";

type Props = {
  components: ComponentState[];
  selectedInstancePath: string | null;
  activeComponentId: string | null;
  onDelete: (component: ComponentState) => Promise<void>;
  onOpenDetail: (component: ComponentState) => Promise<void>;
  onToggleEnabled: (component: ComponentState, enabled: boolean) => Promise<void>;
  tagFilter: string;
  tags: string[];
  query: string;
  authorFilter: string;
  authors: string[];
  onQueryChange: (value: string) => void;
  onAuthorFilterChange: (value: string) => void;
  onTagFilterChange: (tag: string) => void;
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
    tagFilter,
    tags,
    query,
    authorFilter,
    authors,
    onQueryChange,
    onAuthorFilterChange,
    onTagFilterChange,
    styles,
  } = props;

  return (
    <div className={styles.optionsList}>
      <RepositoryFilters
        query={query}
        authorFilter={authorFilter}
        authors={authors}
        tagFilter={tagFilter}
        tags={tags}
        onQueryChange={onQueryChange}
        onAuthorFilterChange={onAuthorFilterChange}
        onTagFilterChange={onTagFilterChange}
        styles={styles}
      />
      {components.length === 0 ? (
        <div className={styles.optionCard}><Text className={styles.empty}>当前筛选条件下没有本地组件。</Text></div>
      ) : components.map((component) => (
        <div
          key={component.id}
          className={`${styles.optionCard} ${styles.catalogItemCard} ${activeComponentId === component.id ? styles.activeOptionCard : ""}`}
        >
          <Text weight="semibold">{component.name}</Text>
          <Text className={styles.empty}>
            {(component.author || "未知作者") + ` · v${component.version ?? 0}`}
          </Text>
          <Text>{component.desc || "无描述"}</Text>
          <Text className={styles.tagPill}>{tagLabel(component.tag)}</Text>
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
                disabled={!selectedInstancePath}
              >
                组件详情
              </Button>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
