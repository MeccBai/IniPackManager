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
            let replacement = if value {
                option.true_result.clone().unwrap_or_else(|| "true".to_string())
            } else {
                option.false_result.clone().unwrap_or_else(|| "false".to_string())
            };
            Ok(placeholders
                .into_iter()
                .map(|placeholder| (placeholder, replacement.clone()))
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
            if index >= option.values.len() {
                return Err(format!("选项 {} 的 enum 下标越界", option.name));
            }
            let replacement = if !option.results.is_empty() {
                if index >= option.results.len() {
                    return Err(format!("选项 {} 的 results 配置不足", option.name));
                }
                option.results[index].clone()
            } else {
                option.values[index].clone()
            };
            Ok(placeholders
                .into_iter()
                .map(|placeholder| (placeholder, replacement.clone()))
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
