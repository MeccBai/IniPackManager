import { Button, Spinner, Text } from "@fluentui/react-components";
import type { DownloadTask } from "../../types";
import { tagLabel } from "../../tagTranslations";

type Props = {
  tasks: DownloadTask[];
  onRetry: (taskId: string) => void;
  onRemove: (taskId: string) => void;
  styles: Record<string, string>;
};

const statusText = {
  queued: "等待下载",
  downloading: "下载并校验中",
  completed: "已导入本地仓库",
  failed: "下载失败",
};

export function DownloadManagerPanel({ tasks, onRetry, onRemove, styles }: Props) {
  const activeCount = tasks.filter((task) => task.status === "queued" || task.status === "downloading").length;
  return (
    <div className={styles.panelBody}>
      <div className={`${styles.optionCard} ${styles.compactInfoCard}`}>
        <Text weight="semibold">下载管理器</Text>
        <Text className={styles.empty}>队列会在后台下载、校验 SHA-256，并导入到目标实例。当前活跃任务：{activeCount}</Text>
      </div>
      {tasks.length === 0 ? (
        <div className={styles.optionCard}><Text className={styles.empty}>暂无下载任务。请在云端仓库中选择组件下载。</Text></div>
      ) : (
        <div className={styles.optionsList}>
          {tasks.map((task) => (
            <div key={task.id} className={`${styles.optionCard} ${styles.catalogItemCard}`}>
              <Text weight="semibold">{task.item.name}</Text>
              <Text className={styles.empty}>{task.item.author || "未知作者"} · {tagLabel(task.item.tag)} · v{task.item.version ?? 0}</Text>
              <Text className={task.status === "failed" ? styles.danger : styles.status}>{statusText[task.status]}</Text>
              <Text className={styles.empty}>目标实例：{task.instance_name}</Text>
              {task.error && <Text className={styles.danger}>{task.error}</Text>}
              <div className={styles.rightAligned}>
                {task.status === "downloading" && <Spinner size="tiny" />}
                {task.status === "failed" && <Button appearance="secondary" onClick={() => onRetry(task.id)}>重试</Button>}
                {task.status !== "downloading" && <Button appearance="subtle" onClick={() => onRemove(task.id)}>移除</Button>}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
