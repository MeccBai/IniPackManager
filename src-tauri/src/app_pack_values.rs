struct ValueOutputParser<'a> {
    expr: &'a str,
    bytes: &'a [u8],
    index: usize,
    var_value: f64,
}

impl<'a> ValueOutputParser<'a> {
    fn new(expr: &'a str, var_value: f64) -> Self {
        Self {
            expr,
            bytes: expr.as_bytes(),
            index: 0,
            var_value,
        }
    }

    fn parse(mut self) -> Result<f64, String> {
        let value = self.parse_expression()?;
        self.skip_whitespace();
        if self.index < self.bytes.len() {
            return Err(format!("第 {} 列附近存在非法字符", self.index + 1));
        }
        if !value.is_finite() {
            return Err("计算结果非法（溢出或非数字）".to_string());
        }
        Ok(value)
    }

    fn parse_expression(&mut self) -> Result<f64, String> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            if self.consume(b'+') {
                value += self.parse_term()?;
            } else if self.consume(b'-') {
                value -= self.parse_term()?;
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            if self.consume(b'*') {
                value *= self.parse_factor()?;
            } else if self.consume(b'/') {
                let divisor = self.parse_factor()?;
                if divisor == 0.0 {
                    return Err(format!("第 {} 列出现除以 0", self.index + 1));
                }
                value /= divisor;
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        if self.consume(b'+') {
            return self.parse_factor();
        }
        if self.consume(b'-') {
            return Ok(-self.parse_factor()?);
        }
        if self.consume(b'(') {
            let value = self.parse_expression()?;
            self.skip_whitespace();
            if !self.consume(b')') {
                return Err(format!("第 {} 列缺少右括号", self.index + 1));
            }
            return Ok(value);
        }
        if self.consume_var() {
            return Ok(self.var_value);
        }
        self.parse_number()
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let start = self.index;
        let mut has_digit = false;

        while self.current().is_some_and(|ch| ch.is_ascii_digit()) {
            has_digit = true;
            self.index += 1;
        }

        if self.current() == Some(b'.') {
            self.index += 1;
            while self.current().is_some_and(|ch| ch.is_ascii_digit()) {
                has_digit = true;
                self.index += 1;
            }
        }

        if !has_digit {
            return Err(format!("第 {} 列需要数字或 var", start + 1));
        }

        let raw = &self.expr[start..self.index];
        raw.parse::<f64>()
            .map_err(|_| format!("第 {} 列数字格式错误: {}", start + 1, raw))
    }

    fn consume_var(&mut self) -> bool {
        let start = self.index;
        if self.bytes.len().saturating_sub(start) < 3 {
            return false;
        }
        let head = &self.bytes[start..start + 3];
        if !head.eq_ignore_ascii_case(b"var") {
            return false;
        }
        let next = self.bytes.get(start + 3).copied();
        if next.is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == b'_') {
            return false;
        }
        self.index += 3;
        true
    }

    fn consume(&mut self, expected: u8) -> bool {
        if self.current() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn current(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn skip_whitespace(&mut self) {
        while self.current().is_some_and(|ch| ch.is_ascii_whitespace()) {
            self.index += 1;
        }
    }
}

fn format_value_output_number(value: f64) -> String {
    let rounded = value.round();
    if (value - rounded).abs() < 1e-9
        && rounded >= i64::MIN as f64
        && rounded <= i64::MAX as f64
    {
        return (rounded as i64).to_string();
    }
    let mut text = format!("{value:.10}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn normalize_list_values(raw_list: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in raw_list {
        for part in raw.split(',') {
            let text = part.trim();
            if !text.is_empty() {
                out.push(text.to_string());
            }
        }
    }
    out
}

fn option_placeholders(option: &RawPackOption) -> Vec<String> {
    normalize_list_values(&option.placeholders)
}

fn option_value_outputs(option: &RawPackOption) -> Vec<String> {
    normalize_list_values(&option.value_outputs)
}

#[derive(Clone)]
struct EnumOptionSet {
    values: Vec<String>,
    results: Vec<String>,
}

fn enum_indexed_field(
    option: &RawPackOption,
    prefix: &str,
    index: usize,
) -> Result<Option<Vec<String>>, String> {
    let field_name = format!("{prefix}{index}");
    let Some(value) = option.extra.iter().find_map(|(key, value)| {
        key.eq_ignore_ascii_case(&field_name).then_some(value)
    }) else {
        return Ok(None);
    };
    let values = if let Some(text) = value.as_str() {
        vec![text.to_string()]
    } else if let Some(array) = value.as_array() {
        array
            .iter()
            .map(|item| {
                item.as_str()
                    .map(ToString::to_string)
                    .ok_or_else(|| format!("选项 {} 的 {} 必须是字符串或字符串数组", option.name, field_name))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        return Err(format!(
            "选项 {} 的 {} 必须是字符串或字符串数组",
            option.name, field_name
        ));
    };
    Ok(Some(normalize_list_values(&values)))
}

fn enum_option_sets(
    option: &RawPackOption,
    placeholder_count: usize,
) -> Result<Vec<EnumOptionSet>, String> {
    let mut uses_indexed_fields = false;
    for index in 1..=placeholder_count {
        if enum_indexed_field(option, "Results", index)?.is_some() {
            uses_indexed_fields = true;
            break;
        }
    }
    if !uses_indexed_fields {
        return Ok(vec![
            EnumOptionSet {
                values: option.values.clone(),
                results: option.results.clone(),
            };
            placeholder_count
        ]);
    }

    let mut sets = Vec::with_capacity(placeholder_count);
    for index in 1..=placeholder_count {
        let results = enum_indexed_field(option, "Results", index)?.ok_or_else(|| {
            format!("选项 {} 缺少 Results{}，无法匹配第 {} 个占位符", option.name, index, index)
        })?;
        if results.len() != option.values.len() {
            return Err(format!(
                "选项 {} 的 Results{} 项目数 ({}) 必须与 Values ({}) 一致",
                option.name,
                index,
                results.len(),
                option.values.len()
            ));
        }
        sets.push(EnumOptionSet {
            values: option.values.clone(),
            results,
        });
    }
    Ok(sets)
}

fn bool_result_replacements(
    option_name: &str,
    placeholders: &[String],
    configured_results: &[String],
    default_result: &str,
    field_name: &str,
) -> Result<Vec<String>, String> {
    let results = normalize_list_values(configured_results);
    if results.is_empty() {
        return Ok(vec![default_result.to_string(); placeholders.len()]);
    }
    if results.len() == 1 {
        return Ok(vec![results[0].clone(); placeholders.len()]);
    }
    if results.len() != placeholders.len() {
        return Err(format!(
            "选项 {} 的 {} 数量 ({}) 必须与 Placeholders 数量 ({}) 一致",
            option_name,
            field_name,
            results.len(),
            placeholders.len()
        ));
    }
    Ok(results)
}

enum ControlBlock {
    If { include: bool, has_else: bool },
    Enum { include: bool },
}

fn control_selection(
    option: &RawPackOption,
    selections: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if let Some(value) = selections.get(&option.name) {
        return Ok(value.clone());
    }
    option
        .default
        .as_ref()
        .and_then(toml_default_to_json)
        .ok_or_else(|| format!("控制选项 {} 未提供值且缺少默认值", option.name))
}

fn require_control_option<'a>(
    options: &'a HashMap<String, &RawPackOption>,
    name: &str,
    option_type: &str,
    file_name: &str,
    line_number: usize,
) -> Result<&'a RawPackOption, String> {
    let option = options.get(name).ok_or_else(|| {
        format!("{}:{} 引用了不存在的控制选项 {}", file_name, line_number, name)
    })?;
    if !option.control {
        return Err(format!(
            "{}:{} 的选项 {} 必须设置 Control = true",
            file_name, line_number, name
        ));
    }
    if !option.option_type.eq_ignore_ascii_case(option_type) {
        return Err(format!(
            "{}:{} 的控制选项 {} 必须是 {} 类型",
            file_name, line_number, name, option_type
        ));
    }
    Ok(option)
}

fn control_block_include(block: &Option<ControlBlock>) -> bool {
    match block {
        None => true,
        Some(ControlBlock::If { include, .. } | ControlBlock::Enum { include }) => *include,
    }
}

fn parse_control_option_name(line: &str, keyword: &str) -> Result<String, String> {
    let value = line
        .strip_prefix(keyword)
        .unwrap_or_default()
        .trim();
    let Some(name) = value.strip_prefix('$') else {
        return Err(format!("{} 后必须使用 $OptionName", keyword));
    };
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(format!("{} 的选项名无效", keyword));
    }
    Ok(name.to_string())
}

fn parse_enum_control(line: &str) -> Result<(String, String), String> {
    let value = line.strip_prefix("#Enum").unwrap_or_default().trim();
    let Some(value) = value.strip_prefix('$') else {
        return Err("#Enum 后必须使用 $EnumName:EnumValue".to_string());
    };
    let Some((name, enum_value)) = value.split_once(':') else {
        return Err("#Enum 后必须使用 $EnumName:EnumValue".to_string());
    };
    if name.is_empty()
        || enum_value.is_empty()
        || name.chars().any(char::is_whitespace)
        || enum_value.chars().any(char::is_whitespace)
    {
        return Err("#Enum 的枚举名或枚举值无效".to_string());
    }
    Ok((name.to_string(), enum_value.to_string()))
}

fn apply_control_blocks(
    content: String,
    options: &HashMap<String, &RawPackOption>,
    selections: &HashMap<String, serde_json::Value>,
    file_name: &str,
) -> Result<String, String> {
    let mut output = String::new();
    let mut block = None;

    for (index, line) in content.split_inclusive('\n').enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.starts_with("#If") {
            if block.is_some() {
                return Err(format!("{}:{} 不允许嵌套控制块", file_name, line_number));
            }
            let name = parse_control_option_name(trimmed, "#If")
                .map_err(|err| format!("{}:{} {}", file_name, line_number, err))?;
            let option = require_control_option(options, &name, "bool", file_name, line_number)?;
            let include = control_selection(option, selections)?
                .as_bool()
                .ok_or_else(|| format!("{}:{} 的控制选项 {} 需要 bool 值", file_name, line_number, name))?;
            block = Some(ControlBlock::If {
                include,
                has_else: false,
            });
            continue;
        }
        if trimmed == "#Else" {
            let Some(ControlBlock::If { include, has_else }) = block.as_mut() else {
                return Err(format!("{}:{} #Else 未处于 #If 块中", file_name, line_number));
            };
            if *has_else {
                return Err(format!("{}:{} 一个 #If 块只能包含一个 #Else", file_name, line_number));
            }
            *include = !*include;
            *has_else = true;
            continue;
        }
        if trimmed == "#EndIf" {
            if !matches!(block, Some(ControlBlock::If { .. })) {
                return Err(format!("{}:{} #EndIf 未匹配 #If", file_name, line_number));
            }
            block = None;
            continue;
        }
        if trimmed.starts_with("#Enum") {
            if block.is_some() {
                return Err(format!("{}:{} 不允许嵌套控制块", file_name, line_number));
            }
            let (name, expected_value) = parse_enum_control(trimmed)
                .map_err(|err| format!("{}:{} {}", file_name, line_number, err))?;
            let option = require_control_option(options, &name, "enum", file_name, line_number)?;
            let value = control_selection(option, selections)?;
            let selected_index = value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
                .map(|value| value as isize)
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|value| option.values.iter().position(|item| item == value))
                        .map(|value| value as isize)
                })
                .ok_or_else(|| format!("{}:{} 的控制选项 {} 需要枚举下标或 Values 中的值", file_name, line_number, name))?;
            if selected_index < 0 || selected_index as usize >= option.values.len() {
                return Err(format!("{}:{} 的控制选项 {} 枚举下标越界", file_name, line_number, name));
            }
            block = Some(ControlBlock::Enum {
                include: option.values[selected_index as usize] == expected_value,
            });
            continue;
        }
        if trimmed == "#EndEnum" {
            if !matches!(block, Some(ControlBlock::Enum { .. })) {
                return Err(format!("{}:{} #EndEnum 未匹配 #Enum", file_name, line_number));
            }
            block = None;
            continue;
        }
        if control_block_include(&block) {
            output.push_str(line);
        }
    }

    if let Some(block) = block {
        let expected_end = match block {
            ControlBlock::If { .. } => "#EndIf",
            ControlBlock::Enum { .. } => "#EndEnum",
        };
        return Err(format!("{} 缺少 {}", file_name, expected_end));
    }
    Ok(output)
}

fn resolve_option_replacements(
    option: &RawPackOption,
    selection: &serde_json::Value,
) -> Result<Vec<(String, String)>, String> {
    let placeholders = option_placeholders(option);
    if placeholders.is_empty() {
        return Err(format!("选项 {} 缺少 placeholders", option.name));
    }
    let value_outputs = option_value_outputs(option);

    match option.option_type.to_lowercase().as_str() {
        "bool" => {
            if !value_outputs.is_empty() {
                return Err(format!("选项 {} 是 bool，不支持 valueOutputs", option.name));
            }
            let value = selection
                .as_bool()
                .ok_or_else(|| format!("选项 {} 需要 bool 值", option.name))?;
            let replacements = if value {
                bool_result_replacements(
                    &option.name,
                    &placeholders,
                    &option.true_results,
                    "true",
                    "TrueResult",
                )?
            } else {
                bool_result_replacements(
                    &option.name,
                    &placeholders,
                    &option.false_results,
                    "false",
                    "FalseResult",
                )?
            };
            Ok(placeholders
                .into_iter()
                .zip(replacements)
                .collect())
        }
        "int" => {
            let value = selection
                .as_i64()
                .or_else(|| selection.as_u64().and_then(|v| i64::try_from(v).ok()))
                .ok_or_else(|| format!("选项 {} 需要 int 值", option.name))?;
            if let Some(min) = option.min {
                if value < min {
                    return Err(format!("选项 {} 不能小于 {}", option.name, min));
                }
            }
            if let Some(max) = option.max {
                if value > max {
                    return Err(format!("选项 {} 不能大于 {}", option.name, max));
                }
            }
            let mut replacements = Vec::new();
            for (index, placeholder) in placeholders.into_iter().enumerate() {
                let replacement = if let Some(expr) = value_outputs.get(index) {
                    let parsed = ValueOutputParser::new(expr, value as f64).parse().map_err(|err| {
                        format!("选项 {} 的 valueOutputs[{}] 无效: {}", option.name, index, err)
                    })?;
                    format_value_output_number(parsed)
                } else {
                    value.to_string()
                };
                replacements.push((placeholder, replacement));
            }
            Ok(replacements)
        }
        "enum" => {
            if !value_outputs.is_empty() {
                return Err(format!("选项 {} 是 enum，不支持 valueOutputs", option.name));
            }
            let index = selection
                .as_i64()
                .or_else(|| selection.as_u64().and_then(|v| i64::try_from(v).ok()))
                .ok_or_else(|| format!("选项 {} 需要 enum 下标值", option.name))?;
            if index < 0 {
                return Err(format!("选项 {} 的 enum 下标非法", option.name));
            }
            let index = index as usize;
            let enum_sets = enum_option_sets(option, placeholders.len())?;
            if index >= enum_sets[0].values.len() {
                return Err(format!("选项 {} 的 enum 下标越界", option.name));
            }
            Ok(placeholders
                .into_iter()
                .zip(enum_sets)
                .map(|(placeholder, set)| {
                    let replacement = set
                        .results
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| set.values[index].clone());
                    (placeholder, replacement)
                })
                .collect())
        }
        other => Err(format!("不支持的选项类型: {}", other)),
    }
}

fn ensure_include_entry(main_ini_path: &Path, include_rel_path: &str) -> Result<(), String> {
    let include_line = format!("+={}", include_rel_path.replace('\\', "/"));
    let include_line_lower = include_line.to_lowercase();

    let mut content = if main_ini_path.exists() {
        fs::read_to_string(main_ini_path)
            .map_err(|err| format!("读取 {} 失败: {err}", main_ini_path.display()))?
    } else {
        String::new()
    };

    if content
        .lines()
        .any(|line| line.trim().to_lowercase() == include_line_lower)
    {
        return Ok(());
    }

    let mut lines: Vec<String> = if content.is_empty() {
        Vec::new()
    } else {
        content.lines().map(|line| line.to_string()).collect()
    };

    let include_section_idx = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("[#include]"));

    match include_section_idx {
        Some(idx) => {
            let insert_idx = lines
                .iter()
                .enumerate()
                .skip(idx + 1)
                .find(|(_, line)| {
                    let trimmed = line.trim();
                    trimmed.starts_with('[') && trimmed.ends_with(']')
                })
                .map(|(line_idx, _)| line_idx)
                .unwrap_or(lines.len());
            lines.insert(insert_idx, include_line);
        }
        None => {
            if !lines.is_empty() && !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            lines.push("[#include]".to_string());
            lines.push(include_line);
        }
    }

    content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    fs::write(main_ini_path, content)
        .map_err(|err| format!("写入 {} 失败: {err}", main_ini_path.display()))
}

fn remove_include_entry(main_ini_path: &Path, include_rel_path: &str) -> Result<(), String> {
    if !main_ini_path.exists() {
        return Ok(());
    }
    let include_line = format!("+={}", include_rel_path.replace('\\', "/")).to_lowercase();
    let content = fs::read_to_string(main_ini_path)
        .map_err(|err| format!("读取 {} 失败: {err}", main_ini_path.display()))?;
    let lines: Vec<String> = content
        .lines()
        .filter(|line| line.trim().to_lowercase() != include_line)
        .map(|line| line.to_string())
        .collect();
    let mut next = lines.join("\n");
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    fs::write(main_ini_path, next)
        .map_err(|err| format!("写入 {} 失败: {err}", main_ini_path.display()))
}

fn instance_key(instance_path: &str) -> String {
    normalize_path_key(instance_path)
}

fn component_id(pack_path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    normalize_path_key(pack_path).hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
