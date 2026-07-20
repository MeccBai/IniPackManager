export type CatalogFilterItem = {
  name: string;
  desc: string;
  author: string;
  tag: string;
};

export type CatalogFilters = {
  query: string;
  author: string;
  tag: string;
};

export function collectCatalogAuthors(items: CatalogFilterItem[]) {
  return [...new Set(items.map((item) => item.author.trim()).filter(Boolean))]
    .sort((left, right) => left.localeCompare(right));
}

export function collectCatalogTags(items: CatalogFilterItem[]) {
  return [...new Set(items.map((item) => item.tag).filter(Boolean))].sort();
}

export function filterCatalogItems<T extends CatalogFilterItem>(items: T[], filters: CatalogFilters) {
  const query = filters.query.trim().toLowerCase();
  return items.filter((item) => {
    if (filters.tag && item.tag !== filters.tag) return false;
    if (filters.author && item.author !== filters.author) return false;
    return !query || [item.name, item.desc, item.author].join("\n").toLowerCase().includes(query);
  });
}
