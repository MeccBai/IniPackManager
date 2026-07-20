import { Button, Input, Select } from "@fluentui/react-components";
import type { ReactNode } from "react";
import { tagLabel } from "../../tagTranslations";

type Props = {
  query: string;
  authorFilter: string;
  authors: string[];
  tagFilter: string;
  tags: string[];
  onQueryChange: (value: string) => void;
  onAuthorFilterChange: (value: string) => void;
  onTagFilterChange: (value: string) => void;
  trailingAction?: ReactNode;
  styles: Record<string, string>;
};

export function RepositoryFilters(props: Props) {
  const { query, authorFilter, authors, tagFilter, tags, onQueryChange, onAuthorFilterChange, onTagFilterChange, trailingAction, styles } = props;
  return (
    <>
      <div className={styles.filterBar}>
        <Input value={query} onChange={(_, data) => onQueryChange(data.value)} placeholder="搜索名字、简介、作者" />
        <Select value={authorFilter} onChange={(event) => onAuthorFilterChange(event.target.value)}>
          <option value="">全部作者</option>
          {authors.map((author) => <option key={author} value={author}>{author}</option>)}
        </Select>
        {trailingAction}
      </div>
      <div className={styles.tagFilterBar}>
        <Button appearance={tagFilter ? "secondary" : "primary"} size="small" onClick={() => onTagFilterChange("")}>全部 Tag</Button>
        {tags.map((tag) => <Button key={tag} appearance={tagFilter === tag ? "primary" : "secondary"} size="small" onClick={() => onTagFilterChange(tag)}>{tagLabel(tag)}</Button>)}
      </div>
    </>
  );
}
