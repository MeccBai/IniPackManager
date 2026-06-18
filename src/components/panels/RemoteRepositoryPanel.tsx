import { Button, Input, Select, Spinner, Text } from "@fluentui/react-components";
import type { RemotePackageSummary } from "../../types";

type Props = {
  game: string;
  catalogName: string;
  catalogDesc: string;
  loading: boolean;
  query: string;
  authorFilter: string;
  authors: string[];
  packages: RemotePackageSummary[];
  importingUrl: string | null;
  onQueryChange: (value: string) => void;
  onAuthorFilterChange: (value: string) => void;
  onRefresh: () => Promise<void>;
  onImport: (item: RemotePackageSummary) => Promise<void>;
  styles: Record<string, string>;
};

export function RemoteRepositoryPanel(props: Props) {
  const {
    game,
    catalogName,
    catalogDesc,
    loading,
    query,
    authorFilter,
    authors,
    packages,
    importingUrl,
    onQueryChange,
    onAuthorFilterChange,
    onRefresh,
    onImport,
    styles,
  } = props;

  return (
    <div className={styles.panelBody}>
      <div className={styles.filterBar}>
        <Input
          value={query}
          onChange={(_, data) => onQueryChange(data.value)}
          placeholder="搜索名字、简介、作者"
        />
        <Select value={authorFilter} onChange={(event) => onAuthorFilterChange(event.target.value)}>
          <option value="">全部作者</option>
          {authors.map((author) => (
            <option key={author} value={author}>
              {author}
            </option>
          ))}
        </Select>
        <Button appearance="secondary" onClick={() => void onRefresh()} disabled={loading}>
          {loading ? "刷新中..." : "刷新列表"}
        </Button>
      </div>

      <div className={`${styles.optionCard} ${styles.compactInfoCard}`}>
        <Text weight="semibold">{catalogName || "云端仓库"}</Text>
        {catalogDesc && <Text className={styles.empty}>{catalogDesc}</Text>}
        <Text className={styles.empty}>当前 Preset/Game：{game || "未设置"}</Text>
      </div>

      {loading ? (
        <Spinner label="正在加载云端组件..." />
      ) : packages.length === 0 ? (
        <div className={styles.optionCard}>
          <Text className={styles.empty}>当前筛选条件下没有可导入组件。</Text>
        </div>
      ) : (
        <div className={styles.optionsList}>
          {packages.map((item) => {
            const disabled = Boolean(item.incompatible_reason) || !item.url.trim();
            return (
              <div key={`${item.id || item.name}-${item.url}`} className={styles.optionCard}>
                <Text weight="semibold">{item.name}</Text>
                <Text className={styles.empty}>
                  {(item.author || "未知作者") + ` · v${item.version ?? 0}`}
                </Text>
                <Text>{item.desc || "无描述"}</Text>
                {item.min_version && <Text className={styles.empty}>最低版本：{item.min_version}</Text>}
                {item.incompatible_reason && <Text className={styles.danger}>{item.incompatible_reason}</Text>}
                {!item.url.trim() && <Text className={styles.danger}>缺少下载地址，无法导入。</Text>}
                <div className={styles.rightAligned}>
                  <Button
                    appearance="primary"
                    onClick={() => void onImport(item)}
                    disabled={disabled || importingUrl === item.url}
                  >
                    {importingUrl === item.url ? "导入中..." : "导入到当前实例"}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
