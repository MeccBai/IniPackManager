# config.toml 配置规则文档

> 本文档描述了 IniPackManager 中组件包（Pack）的 `config.toml` 配置文件的完整规则。

## 概述

`config.toml` 是每个组件包的核心配置文件，定义了组件包的元信息、用户可配置选项、数据文件映射、依赖关系等。组件包可以是一个本地文件夹，也可以是一个 `.zip` 压缩包。

## 整体结构

```toml
[Config]
# 组件包元信息

[Requirements]
# 依赖要求

[[Options]]
# 用户可配置选项（可以有多个）

[Data]
# 数据文件分组（Rules / Art / Ai / Theme / Ui）

[[Resource]]
# 需要部署的额外资源文件（可以有多个）
```

---

## 1. [Config] — 组件包元信息

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `Name` | String | **是** | 组件包显示名称 |
| `Desc` | String | 否 | 组件包描述说明 |
| `Dir` | String | 否 | 输出子目录。组件文件会被输出到 `Pack/{Dir}/` 下，为空则直接输出到 `Pack/` |
| `Id` | String | 否 | 组件包唯一标识符。用于依赖匹配和组件去重，忽略大小写 |
| `Game` | String | 否 | 目标游戏 ID（如 `mo`、`yr`），启用时校验与实例 Preset 是否匹配 |
| `Version` | Integer | 否 | 组件包的版本号，默认 `0` |

### 示例

```toml
[Config]
Name = "我的规则包"
Desc = "这是一个示例组件包"
Dir = "MyMod"
Id = "my_rules_pack"
Game = "mo"
Version = 2
```

---

## 2. [Requirements] — 依赖要求

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `Files` | String 数组 | 否 | 实例目录下必须存在的文件列表（相对路径）。任意文件缺失则阻止启用 |
| `Pack` | String 数组 | 否 | 必须已启用其他组件包的 ID 列表。匹配时忽略大小写 |
| `MinVersion` | String | 否 | 管理器最低版本要求（语义化版本号，如 `1.0.0`） |

### 示例

```toml
[Requirements]
Files = ["game.exe", "Resources/custom.mix"]
Pack = ["core_library", "patch_01"]
MinVersion = "1.2.0"
```

### 依赖检查规则

- **Files**: 检查文件是否存在于实例根目录。
- **Pack**: 遍历当前实例已**启用**的组件列表，比较它们 `[Config].Id` 是否匹配。当前正在操作的组件自身不计入检查。
- **MinVersion**: 比较管理器的当前版本与要求版本，管理器版本低于要求时拒绝安装/启用。
- **Game**: 在 `[Config]` 中指定的 `Game` 字段也会参与校验，如果设置则必须与实例的 Preset ID 一致。

---

## 3. [[Options]] — 用户可配置选项

一个组件包可以定义多个选项，每个选项对应一个 `[[Options]]` 表。选项支持三种类型：`bool`、`int`、`enum`。

### 通用字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `Name` | String | **是** | 选项名称，用于在 Data 中引用 |
| `Desc` | String | 否 | 选项描述，为空时默认显示 `Name` |
| `Type` | String | **是** | 选项类型：`bool` / `int` / `enum` |
| `Control` | Boolean | 否 | 设为 `true` 时用于控制条件块，不参与占位符替换，`Placeholders` 及各类替换结果字段会被忽略 |
| `Placeholders` | String 或 String 数组 | 条件必填 | 数据文件中要被替换的占位符。`Control = true` 时忽略该字段，否则必填 |
| `Default` | 见下方说明 | 否 | 默认值。格式取决于选项类型 |

### 3.1 bool 类型

用于开关型选项。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `TrueResult` | String 或 String 数组 | 否 | 选中时替换占位符的值，默认为 `"true"` |
| `FalseResult` | String 或 String 数组 | 否 | 取消选中时替换占位符的值，默认为 `"false"` |
| `Default` | Boolean 或 `[Boolean]` | 否 | 默认值。可以写成 `true` / `false` 或 `[true]` / `[false]` |

**注意**: bool 类型**不支持** `ValueOutputs` 字段。

#### 示例

```toml
[[Options]]
Name = "EnableFeature"
Desc = "启用新功能"
Type = "bool"
Placeholders = "{ENABLE_FEATURE}"
TrueResult = "1"
FalseResult = "0"
Default = true
```

多个占位符可使用数组分别指定替换结果，数组长度必须与 `Placeholders` 一致；单个字符串会兼容地应用到所有占位符。

```toml
[[Options]]
Name = "DebugMode"
Type = "bool"
Placeholders = ["{DEBUG_MODE}", "{DEBUG_LEVEL}"]
TrueResult = ["enabled", "verbose"]
FalseResult = ["disabled", "quiet"]
Default = [false]
```

### 3.2 int 类型

用于整数值选项。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `Min` | Integer 或 `[Integer]` | 否 | 最小值限制 |
| `Max` | Integer 或 `[Integer]` | 否 | 最大值限制 |
| `ValueOutputs` | String 或 String 数组 | 否 | 输出表达式数组。支持四则运算表达式，`var` 代表用户选中的值 |
| `Default` | Integer 或 `[Integer]` | 否 | 默认值 |

#### ValueOutputs 表达式语法

- 支持运算符：`+`、`-`、`*`、`/`
- 支持括号 `( )`
- 一元正负号 `+`、`-`
- 变量 `var`（大小写不敏感）
- 结果为浮点数，若非整数则保留小数（去除末尾多余的零）

#### 示例

```toml
[[Options]]
Name = "DamageMultiplier"
Desc = "伤害倍率"
Type = "int"
Placeholders = "{DAMAGE_MULT}"
Min = 1
Max = 10
Default = 5
ValueOutputs = "var * 10"
```

```toml
[[Options]]
Name = "MaxUnits"
Type = "int"
Placeholders = ["{MAX_UNITS}", "{MAX_UNITS_HALF}"]
Default = [100]
Min = 10
Max = [500]
ValueOutputs = ["var", "var / 2"]
```

### 3.3 enum 类型

用于枚举选项。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `Values` | String 或 String 数组 | **是** | 枚举项列表 |
| `Results` | String 或 String 数组 | 否 | 各枚举项对应的替换值。不指定时直接使用 `Values` 中的值 |
| `Default` | Integer、String 或 `[Integer]`、`[String]` | 否 | 默认值。数字为下标（0-based），字符串按名称匹配 |

当 `Placeholders` 包含多个占位符时，可以通过 `ResultsN` 为每个占位符定义独立的输出映射，`N` 从 `1` 开始。例如 `Results1` 对应第一个占位符，`Results2` 对应第二个。所有占位符共享同一个 `Values` 枚举列表，每个 `ResultsN` 的项目数必须与 `Values` 一致；使用这套写法时，每个占位符都必须提供对应的 `ResultsN`，否则配置解析会报错。

**注意**: enum 类型**不支持** `ValueOutputs` 字段。

#### 示例

```toml
[[Options]]
Name = "Difficulty"
Desc = "游戏难度"
Type = "enum"
Placeholders = "{DIFFICULTY}"
Values = ["Easy", "Normal", "Hard"]
Results = ["0.5", "1.0", "2.0"]
Default = "Normal"
```

使用下标指定默认值：

```toml
[[Options]]
Name = "ColorScheme"
Type = "enum"
Placeholders = "{COLOR}"
Values = ["Red", "Blue", "Green"]
Default = 1   # 默认为 "Blue"
```

多个占位符使用独立映射：

```toml
[[Options]]
Name = "WeaponMode"
Type = "enum"
Placeholders = ["{WEAPON_NAME}", "{WEAPON_DAMAGE}"]
Values = ["Cannon", "Laser"]
Results1 = ["CANNON", "LASER"]
Results2 = ["90", "45"]
Default = 0
```

### 3.4 Control 条件块

`bool` 和 `enum` 选项可设为 `Control = true`，用于控制 Data 文件中一段文本是否保留。控制选项不需要 `Placeholders`，也不使用 `TrueResult`、`FalseResult`、`Results` 或 `ResultsN`。

控制指令必须独占一行，不能嵌套；管理器会删除所有控制指令行。指令解析发生在默认占位符和普通选项替换之前。

```toml
[[Options]]
Name = "EnableElite"
Type = "bool"
Control = true
Default = false

[[Options]]
Name = "WeaponMode"
Type = "enum"
Control = true
Values = ["Cannon", "Laser"]
Default = "Cannon"
```

```ini
#If $EnableElite
[EliteTank]
Strength=800
#Else
[EliteTank]
Strength=500
#EndIf

#Enum $WeaponMode:Laser
[LaserTank]
Weapon=LaserBeam
#EndEnum
```

`#If $OptionName` 会在对应 bool 选项为 `true` 时保留区块；`#Else` 可选且每个区块最多一次。`#Enum $EnumName:EnumValue` 会在 enum 选项当前值等于 `Values` 中的 `EnumValue` 时保留区块。`#If`/`#Else`/`#EndIf` 与 `#Enum`/`#EndEnum` 不能互相嵌套，选项不存在、未设置 `Control = true`、类型不匹配或指令未闭合都会报错。

---

## 4. [Data] — 数据文件分组

`[Data]` 段包含五个子分组，分别对应游戏的不同 INI 文件类型：

| 分组 | 对应的主 INI 文件 | 说明 |
|------|-------------------|------|
| `[Data.Rules]` | `RulesMain.ini` | 规则文件 |
| `[Data.Art]` | `ArtMain.ini` | 图像/美术文件 |
| `[Data.Ai]` | `AiMain.ini` | AI 文件 |
| `[Data.Theme]` | `ThemeMain.ini` | 主题文件 |
| `[Data.Ui]` | `UIMain.ini` | UI 文件 |

每个分组下的条目格式如下：

```toml
[Data.Rules]
items = [
  { Name = "...", File = "...", Options = ["..."], NeedInclude = true/false }
]
```

但实际 TOML 语法中，每个子分组是一个数组：

```toml
[Data.Rules]
# 方式一：内联表
[[Data.Rules]]
Name = "MyRules"
File = "my_rules.ini"
Options = ["EnableFeature"]
NeedInclude = true

[[Data.Rules]]
Name = "ExtraRules"
File = "extra_rules.ini"
Options = ["Difficulty"]
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `Name` | String | 否* | 数据项名称。如果未提供 `File`，则用作文件名 |
| `File` | String | 否* | 数据文件名（相对于组件包目录）。`Name` 和 `File` 至少填一个 |
| `Options` | String 数组 | 否 | 此文件引用的选项名称列表。对应 `[[Options]]` 中的 `Name` |
| `NeedInclude` | Boolean | 否 | 是否需要在主 INI 文件的 `[#include]` 段中注册。默认 `true` |

> **\*** `Name` 和 `File` 至少提供一个。优先使用 `File`；若 `File` 为空，则使用 `Name` 作为文件名。

### 引用选项的替换过程

1. 组件包的文件（包括 INI 文件中的占位符）被复制到输出目录 `Pack/{Dir}/`
2. 对于每个数据文件，遍历其 `Options` 列表
3. 对每个选项，获取用户选择的值（或默认值）
4. 计算替换结果：将文件内容中的占位符替换为实际值
5. 如果 `NeedInclude = true`，在对应的主 INI 文件（`RulesMain.ini` 等）的 `[#include]` 段中添加注册行

### 示例

```toml
[Data.Rules]
[[Data.Rules]]
Name = "CoreRules"
File = "core.ini"
Options = ["DamageMultiplier", "EnableFeature"]
NeedInclude = true

[[Data.Rules]]
Name = "ExtraRules"
File = "extra.ini"
Options = []
NeedInclude = true

[Data.Art]
[[Data.Art]]
Name = "CustomArt"
File = "custom_art.ini"
Options = ["ColorScheme"]
```

---

## 5. 默认占位符

除了 `[[Options]]` 自定义的占位符，管理器会在每个 `[Data.*]` 数据文件中自动替换以下占位符，再处理选项占位符：

| 占位符 | 替换结果 |
|--------|----------|
| `{Dir}` | 当前组件的输出目录：`Pack/{Config.Dir}`；当 `Config.Dir` 为空时为 `Pack` |
| `{Id}` | 当前组件的 `[Config].Id` 值 |

例如，`Dir = "ACS"` 时，`{Dir}/assets.mix` 会展开为 `Pack/ACS/assets.mix`。

---

## 6. [[Resource]] — 资源文件

一个组件包可以定义多个 `[[Resource]]` 条目。每个条目指定组件包中的一个文件，以及该文件部署到游戏实例的位置。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `File` | String | **是** | 组件包内资源文件的相对路径，不能使用绝对路径或 `..` |
| `Dir` | Boolean | 否 | 是否部署到 `Pack/{Config.Dir}/`。默认 `false`；为 `false` 时部署到游戏根目录 |

```toml
[[Resource]]
File = "Phobos.dll"
Dir = false

[[Resource]]
File = "assets/custom.mix"
Dir = true
```

上例会将 `Phobos.dll` 复制到游戏根目录，将 `assets/custom.mix` 复制到 `Pack/{Config.Dir}/assets/custom.mix`。禁用组件时，管理器会删除这些由 `[[Resource]]` 指定的目标文件。

---

## 7. 完整示例

```toml
[Config]
Name = "Advanced Combat System"
Desc = "增强战斗系统 v2"
Id = "acs_v2"
Game = "mo"
Dir = "ACS"
Version = 3

[Requirements]
Files = ["game.exe", "expandmd03.mix"]
Pack = ["core_pack", "fx_pack"]
MinVersion = "1.0.0"

[[Options]]
Name = "EnableArmor"
Type = "bool"
Placeholders = "{ENABLE_ARMOR}"
TrueResult = "1"
FalseResult = "0"
Default = true

[[Options]]
Name = "ArmorRating"
Type = "int"
Placeholders = ["{ARMOR_RATING}", "{ARMOR_PERCENT}"]
Min = 0
Max = 100
Default = 50
ValueOutputs = ["var", "var * 10"]

[[Options]]
Name = "ArmorType"
Type = "enum"
Placeholders = "{ARMOR_TYPE}"
Values = ["Light", "Medium", "Heavy"]
Results = ["0.5", "1.0", "2.0"]
Default = "Medium"

[Data.Rules]
[[Data.Rules]]
Name = "ArmorRules"
File = "armor_rules.ini"
Options = ["EnableArmor", "ArmorRating", "ArmorType"]
NeedInclude = true

[Data.Art]
[[Data.Art]]
Name = "ArmorArt"
File = "armor_art.ini"
Options = ["ArmorType"]
NeedInclude = true

[[Resource]]
File = "assets/armor.mix"
Dir = true

[[Resource]]
File = "ArmorRender.dll"
Dir = false
```

---

## 8. 注意事项

1. **Id 匹配规则**: `[Config].Id` 在依赖匹配时统一转为小写进行比较。
2. **路径安全**: 文件路径中禁止出现 `..`（上级目录）或绝对路径，所有路径必须为相对路径。
3. **主 INI 文件**: 管理器会在实例目录自动创建 `RulesMain.ini`、`ArtMain.ini`、`AiMain.ini`、`ThemeMain.ini`、`UIMain.ini` 和 `Pack/` 目录。
4. **`[#include]` 注册**: `NeedInclude = true` 的文件会在主 INI 文件中以 `+=` 语法注册。重复注册不会重复添加。
5. **默认值格式**: `Default` 字段支持直接值和单元素数组两种写法，例如 `Default = 5` 等价于 `Default = [5]`，`Default = true` 等价于 `Default = [true]`。
6. **输出目录**: 组件包的所有文件会先复制到输出目录，再对 INI 文件执行占位符替换，最后选择性注册到主 INI 文件。
