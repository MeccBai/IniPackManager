# Ini Pack Manager

用于管理 Red Alert 2 / Yuri's Revenge 类 INI 组件包的桌面工具。它基于 Tauri 2、React 和 Rust 构建，负责实例管理、Preset 初始化、组件包导入、选项配置与文件部署。

作者：[MeccBai](https://github.com/MeccBai/)

项目仓库：[MeccBai/IniPackManager](https://github.com/MeccBai/IniPackManager)

## 功能

- 管理多个游戏实例，并为实例关联 Preset。
- 导入本地 ZIP Pack 或从云端仓库下载组件包。
- 启用、禁用、删除组件，并保存每个组件的选项值。
- 将 Data INI 文件写入 `Pack/{Dir}`，自动注册到对应的主 INI 文件。
- 部署 `[[Resource]]` 资源到游戏根目录或 Pack 子目录。
- 支持 `{Dir}`、`{Id}`、普通 Options 占位符及 `Control = true` 条件块。
- 导出或导入当前实例的 Preset、已安装组件、启用状态和选项值。

## 本地仓库

在“全局设置 -> 仓库”中可以配置本地仓库的父目录。应用会使用以下两个子目录：

```text
<本地仓库>/components
<本地仓库>/repository
```

其中 `components` 保存导入组件的工作副本，`repository` 是配置快照导入时按 `[Config].Id` 查找组件包的来源。

## 配置快照

在“全局设置 -> 配置管理”中：

- 导出会在游戏实例目录生成 `IniPackManager.config.json`。
- 快照包含实例 `preset_id`、所有组件的 `Config.Id`、版本、启用状态和选项值。
- 导入时会检查当前实例的 Preset 是否一致，并从本地 `repository` 目录定位所有组件包；缺少组件包时会中止并提示。

导入会替换当前实例保存的组件配置，并重新应用所有启用的组件。

## 开发

前置条件：安装 Node.js、Rust stable 和目标平台所需的 Tauri 构建环境。

```bash
npm install
npm run tauri dev
```

构建前端：

```bash
npm run build
```

检查 Rust 后端：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## Pack 配置

组件包规则、占位符、资源部署和控制块语法见：[config.toml 规则文档](docs/config.toml-规则文档.md)。

更新器发布配置见：[Tauri Updater 文档](docs/updater/README.md)。
