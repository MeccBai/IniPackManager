export const tagTranslations: Record<string, string> = {
  General: "常规",
  Base: "基础",
  Tools: "工具",
  Unit: "单位",
  Feature: "功能",
  Modifier: "修改",
  Major: "大型",
};

export function tagLabel(tag: string) {
  return tagTranslations[tag] ?? (tag || "未分类");
}
