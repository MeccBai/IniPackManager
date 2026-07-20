import { Button, Spinner, Text } from "@fluentui/react-components";
import type { RemotePackageSummary } from "../../types";
import { tagLabel } from "../../tagTranslations";
import { RepositoryFilters } from "./RepositoryFilters";

type Props = {
  game: string;
  catalogName: string;
  catalogDesc: string;
  loading: boolean;
  query: string;
  authorFilter: string;
  authors: string[];
  tagFilter: string;
  tags: string[];
  onTagFilterChange: (tag: string) => void;
  packages: RemotePackageSummary[];
  downloadingUrls: string[];
  onQueryChange: (value: string) => void;
  onAuthorFilterChange: (value: string) => void;
  onRefresh: () => Promise<void>;
  onImport: (item: RemotePackageSummary) => void;
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
    tagFilter,
    tags,
    onTagFilterChange,
    packages,
    downloadingUrls,
    onQueryChange,
    onAuthorFilterChange,
    onRefresh,
    onImport,
    styles,
  } = props;

  return (
    <div className={styles.panelBody}>
      <RepositoryFilters
        query={query}
        authorFilter={authorFilter}
        authors={authors}
        tagFilter={tagFilter}
        tags={tags}
        onQueryChange={onQueryChange}
        onAuthorFilterChange={onAuthorFilterChange}
        onTagFilterChange={onTagFilterChange}
        trailingAction={<Button appearance="secondary" onClick={() => void onRefresh()} disabled={loading}>{loading ? "刷新中..." : "刷新列表"}</Button>}
        styles={styles}
      />

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
            const queued = downloadingUrls.includes(item.url);
            const downloaded = item.local_status === "downloaded";
            const updateAvailable = item.local_status === "update_available";
            const disabled = Boolean(item.incompatible_reason) || !item.url.trim() || queued || downloaded;
            return (
              <div key={`${item.id || item.name}-${item.url}`} className={`${styles.optionCard} ${styles.catalogItemCard}`}>
                <Text weight="semibold">{item.name}</Text>
                <Text className={styles.empty}>
                  {(item.author || "未知作者") + ` · v${item.version ?? 0}`}
                </Text>
                <Text>{item.desc || "无描述"}</Text>
                <Text className={styles.tagPill}>{tagLabel(item.tag)}</Text>
                {downloaded && <Text className={styles.status}>已下载</Text>}
                {updateAvailable && <Text className={styles.status}>可更新</Text>}
                {item.min_version && <Text className={styles.empty}>最低版本：{item.min_version}</Text>}
                {item.incompatible_reason && <Text className={styles.danger}>{item.incompatible_reason}</Text>}
                {!item.url.trim() && <Text className={styles.danger}>缺少下载地址，无法导入。</Text>}
                <div className={styles.rightAligned}>
                  <Button
                    appearance="primary"
                    onClick={() => void onImport(item)}
                    disabled={disabled}
                  >
                    {downloaded ? "已下载" : queued ? "已加入下载" : updateAvailable ? "更新组件" : "加入下载"}
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
