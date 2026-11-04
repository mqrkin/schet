use std::collections::HashMap;
use std::f64::consts::{E, PI, TAU};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Number,
    Ident,
    Function,
    Operator,
    Paren,
    Comment,
    Assign,
}

#[derive(Clone, Debug)]
pub struct HighlightSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub kind: TokenKind,
}

#[derive(Clone, Debug)]
pub struct LineEval {
    pub value: Option<f64>,
    pub formatted: String,
    pub spans: Vec<HighlightSpan>,
}

pub fn evaluate_document(text: &str) -> Vec<LineEval> {
    let mut vars: HashMap<String, f64> = HashMap::new();
    let lines: Vec<&str> = text.split('\n').collect();
    let mut line_values: Vec<Option<f64>> = Vec::with_capacity(lines.len());
    let mut results: Vec<LineEval> = Vec::with_capacity(lines.len());

    for (line_idx, &raw_line) in lines.iter().enumerate() {
        let eval = evaluate_single_line(raw_line, line_idx, &mut vars, &line_values);
        line_values.push(eval.value);
        results.push(eval);
    }

    results
}

pub fn format_number(val: f64) -> String {
    if val.is_nan() {
        return "NaN".to_string();
    }
    if val.is_infinite() {
        return if val.is_sign_positive() { "∞" } else { "-∞" }.to_string();
    }
    let val = if val.abs() < 1e-12 { 0.0 } else { val };

    if val != 0.0 && (val.abs() >= 1e15 || val.abs() < 1e-7) {
        return format!("{:e}", val);
    }

    let rounded = (val * 1e8).round() / 1e8;
    let is_neg = rounded < 0.0;
    let abs_val = rounded.abs();

    let s = format!("{:.8}", abs_val);
    let mut parts = s.split('.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next().unwrap_or("").trim_end_matches('0');

    let mut grouped = String::with_capacity(int_part.len() + int_part.len() / 3 + 2);
    if is_neg {
        grouped.push('-');
    }
    for (i, ch) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i) % 3 == 0 {
            grouped.push(' ');
        }
        grouped.push(ch);
    }

    if !frac_part.is_empty() {
        grouped.push('.');
        grouped.push_str(frac_part);
    }

    grouped
}

fn is_known_function(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "sqrt"
            | "корень"
            | "cbrt"
            | "abs"
            | "модуль"
            | "round"
            | "окр"
            | "floor"
            | "ceil"
            | "trunc"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "sinh"
            | "cosh"
            | "tanh"
            | "ln"
            | "log"
            | "log2"
            | "log10"
            | "exp"
            | "min"
            | "мин"
            | "max"
            | "макс"
            | "pow"
            | "степень"
            | "avg"
            | "mean"
            | "среднее"
            | "срзнач"
            | "sum"
            | "total"
            | "сумма"
            | "итого"
            | "fact"
            | "deg"
            | "rad"
    )
}

fn is_pure_math_rhs(after: &str) -> bool {
    let trimmed = after.trim();
    if trimmed.is_empty() {
        return false;
    }
    let Some(first) = trimmed.chars().next() else {
        return false;
    };
    if !(first.is_ascii_digit() || first == '(' || first == '+' || first == '-') {
        return false;
    }
    match tokenize(trimmed) {
        Ok(toks) if !toks.is_empty() => toks.iter().all(|t| {
            !matches!(t, LexToken::Ident(s) if !is_known_function(s) && s != "pi" && s != "π" && s != "пи" && s != "e")
        }),
        _ => false,
    }
}

fn find_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            let before = line[..i].trim();
            if before.is_empty() {
                return Some(i);
            }
            if bytes[i - 1].is_ascii_whitespace() {
                let after = &line[i + 2..];
                if !is_pure_math_rhs(after) {
                    return Some(i);
                }
            }
        }
        if bytes[i] == b'#' {
            let next_is_digit = i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit();
            if !next_is_digit {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_thousand_space_at(chars: &[(usize, char)], pos: usize) -> bool {
    if pos >= chars.len() {
        return false;
    }
    let ch = chars[pos].1;
    if ch != ' ' && ch != '\u{00a0}' && ch != '\u{202f}' {
        return false;
    }
    if pos + 3 >= chars.len() {
        return false;
    }
    let d1 = chars[pos + 1].1.is_ascii_digit();
    let d2 = chars[pos + 2].1.is_ascii_digit();
    let d3 = chars[pos + 3].1.is_ascii_digit();
    let d4 = pos + 4 < chars.len() && chars[pos + 4].1.is_ascii_digit();
    d1 && d2 && d3 && !d4
}

pub fn highlight_line(line: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let comment_pos = find_comment_start(line);
    let code_part = match comment_pos {
        Some(pos) => &line[..pos],
        None => line,
    };

    let mut idx = 0;
    let chars: Vec<(usize, char)> = code_part.char_indices().collect();
    while idx < chars.len() {
        let (start_b, ch) = chars[idx];
        if ch.is_whitespace() {
            idx += 1;
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && idx + 1 < chars.len() && chars[idx + 1].1.is_ascii_digit()) {
            let mut end_idx = idx + 1;
            let mut seen_dot = ch == '.';
            while end_idx < chars.len() {
                let c = chars[end_idx].1;
                if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '%' {
                    if c == '.' {
                        seen_dot = true;
                    }
                    end_idx += 1;
                } else if c == ','
                    && !seen_dot
                    && end_idx + 1 < chars.len()
                    && chars[end_idx + 1].1.is_ascii_digit()
                {
                    seen_dot = true;
                    end_idx += 1;
                } else if !seen_dot && is_thousand_space_at(&chars, end_idx) {
                    end_idx += 4;
                } else {
                    break;
                }
            }
            let end_b = if end_idx < chars.len() {
                chars[end_idx].0
            } else {
                code_part.len()
            };
            spans.push(HighlightSpan {
                start_byte: start_b,
                end_byte: end_b,
                kind: TokenKind::Number,
            });
            idx = end_idx;
        } else if ch.is_alphabetic() || ch == '_' || ch == 'π' || ch == '#' {
            let mut end_idx = idx + 1;
            while end_idx < chars.len() {
                let c = chars[end_idx].1;
                if c.is_alphanumeric() || c == '_' {
                    end_idx += 1;
                } else {
                    break;
                }
            }
            let end_b = if end_idx < chars.len() {
                chars[end_idx].0
            } else {
                code_part.len()
            };
            let word = &code_part[start_b..end_b];
            let kind = if is_known_function(word) {
                TokenKind::Function
            } else {
                TokenKind::Ident
            };
            spans.push(HighlightSpan {
                start_byte: start_b,
                end_byte: end_b,
                kind,
            });
            idx = end_idx;
        } else if ch == '(' || ch == ')' {
            spans.push(HighlightSpan {
                start_byte: start_b,
                end_byte: start_b + ch.len_utf8(),
                kind: TokenKind::Paren,
            });
            idx += 1;
        } else if ch == '=' || ch == ':' {
            spans.push(HighlightSpan {
                start_byte: start_b,
                end_byte: start_b + ch.len_utf8(),
                kind: TokenKind::Assign,
            });
            idx += 1;
        } else {
            spans.push(HighlightSpan {
                start_byte: start_b,
                end_byte: start_b + ch.len_utf8(),
                kind: TokenKind::Operator,
            });
            idx += 1;
        }
    }

    if let Some(pos) = comment_pos {
        spans.push(HighlightSpan {
            start_byte: pos,
            end_byte: line.len(),
            kind: TokenKind::Comment,
        });
    }

    spans
}

fn preceding_block_values(line_values: &[Option<f64>]) -> Vec<f64> {
    let mut block = Vec::new();
    let mut i = line_values.len();
    while i > 0 {
        i -= 1;
        if let Some(v) = line_values[i] {
            block.push(v);
        } else if !block.is_empty() {
            break;
        }
    }
    if block.is_empty() {
        for v in line_values.iter().flatten() {
            block.push(*v);
        }
    }
    block
}

fn evaluate_single_line(
    raw_line: &str,
    _line_idx: usize,
    vars: &mut HashMap<String, f64>,
    line_values: &[Option<f64>],
) -> LineEval {
    let spans = highlight_line(raw_line);
    let code_slice = match find_comment_start(raw_line) {
        Some(pos) => &raw_line[..pos],
        None => raw_line,
    }
    .trim();

    if code_slice.is_empty() {
        return LineEval {
            value: None,
            formatted: String::new(),
            spans,
        };
    }

    let mut assign_target: Option<(String, Option<char>)> = None;
    let mut expr_str = code_slice;

    if let Some(eq_idx) = find_top_level_assign(code_slice) {
        let lhs = code_slice[..eq_idx].trim();
        let rhs = code_slice[eq_idx + 1..].trim();
        let (var_name, compound_op) = if let Some(stripped) = lhs.strip_suffix('+') {
            (stripped.trim(), Some('+'))
        } else if let Some(stripped) = lhs.strip_suffix('-') {
            (stripped.trim(), Some('-'))
        } else if let Some(stripped) = lhs.strip_suffix('*') {
            (stripped.trim(), Some('*'))
        } else if let Some(stripped) = lhs.strip_suffix('/') {
            (stripped.trim(), Some('/'))
        } else {
            (lhs, None)
        };

        if is_valid_identifier(var_name) && !rhs.is_empty() {
            assign_target = Some((var_name.to_lowercase(), compound_op));
            expr_str = rhs;
        }
    } else if let Some(colon_idx) = code_slice.find(':') {
        let lhs = code_slice[..colon_idx].trim();
        let rhs = code_slice[colon_idx + 1..].trim();
        if !rhs.is_empty() {
            if is_valid_identifier(lhs) {
                assign_target = Some((lhs.to_lowercase(), None));
            }
            expr_str = rhs;
        }
    }

    match eval_expr(expr_str, vars, line_values) {
        Ok(mut val) => {
            if let Some((var_name, compound_op)) = assign_target {
                if let Some(op) = compound_op {
                    let prev = vars.get(&var_name).copied().unwrap_or(0.0);
                    val = match op {
                        '+' => prev + val,
                        '-' => prev - val,
                        '*' => prev * val,
                        '/' => prev / val,
                        _ => val,
                    };
                }
                vars.insert(var_name, val);
            }
            LineEval {
                value: Some(val),
                formatted: format_number(val),
                spans,
            }
        }
        Err(_err) => LineEval {
            value: None,
            formatted: String::new(),
            spans,
        },
    }
}

fn find_top_level_assign(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    for i in 0..bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            b'=' if depth == 0 => {
                let prev = if i > 0 { bytes[i - 1] } else { 0 };
                let next = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
                if prev != b'=' && prev != b'!' && prev != b'<' && prev != b'>' && next != b'=' {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

// --- Lexer & Recursive Descent Parser ---

#[derive(Clone, Debug, PartialEq)]
enum LexToken {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    IntDiv,
    Percent,
    Caret,
    Bang,
    Deg,
    LParen,
    RParen,
    Comma,
    Of,
}

#[derive(Clone, Copy, Debug)]
enum Val {
    Plain(f64),
    Percent(f64),
}

impl Val {
    fn as_f64(self) -> f64 {
        match self {
            Val::Plain(v) | Val::Percent(v) => v,
        }
    }
}

fn is_decimal_comma(chars: &[char], pos: usize, in_fn_call: bool) -> bool {
    if pos + 1 >= chars.len() || !chars[pos + 1].is_ascii_digit() {
        return false;
    }
    if !in_fn_call {
        return true;
    }
    // Inside a function call `fn(...)`: if there is another comma at the same paren depth
    // before the closing `)` (e.g. `round(3,1415926 * 2, 3)`), then `3,1415926` is a decimal comma!
    let mut depth = 0usize;
    let mut j = pos + 1;
    while j < chars.len() {
        match chars[j] {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            ',' | ';' if depth == 0 => {
                return true;
            }
            _ => {}
        }
        j += 1;
    }
    false
}

fn is_char_thousand_space(chars: &[char], pos: usize) -> bool {
    if pos >= chars.len() {
        return false;
    }
    let ch = chars[pos];
    if ch != ' ' && ch != '\u{00a0}' && ch != '\u{202f}' {
        return false;
    }
    if pos + 3 >= chars.len() {
        return false;
    }
    let d1 = chars[pos + 1].is_ascii_digit();
    let d2 = chars[pos + 2].is_ascii_digit();
    let d3 = chars[pos + 3].is_ascii_digit();
    let d4 = pos + 4 < chars.len() && chars[pos + 4].is_ascii_digit();
    d1 && d2 && d3 && !d4
}

fn tokenize(input: &str) -> Result<Vec<LexToken>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    // Track whether each open parenthesis belongs to a function call `fn(...)`
    let mut fn_paren_stack: Vec<bool> = Vec::new();

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        let in_fn_call = fn_paren_stack.last().copied().unwrap_or(false);

        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            if c == '0' && i + 1 < chars.len() && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                i += 2;
                let mut hex_s = String::new();
                while i < chars.len() && (chars[i].is_ascii_hexdigit() || chars[i] == '_') {
                    if chars[i] != '_' {
                        hex_s.push(chars[i]);
                    }
                    i += 1;
                }
                let v = u64::from_str_radix(&hex_s, 16).map_err(|e| e.to_string())?;
                tokens.push(LexToken::Num(v as f64));
                continue;
            }
            if c == '0' && i + 1 < chars.len() && (chars[i + 1] == 'b' || chars[i + 1] == 'B') {
                i += 2;
                let mut bin_s = String::new();
                while i < chars.len() && (chars[i] == '0' || chars[i] == '1' || chars[i] == '_') {
                    if chars[i] != '_' {
                        bin_s.push(chars[i]);
                    }
                    i += 1;
                }
                let v = u64::from_str_radix(&bin_s, 2).map_err(|e| e.to_string())?;
                tokens.push(LexToken::Num(v as f64));
                continue;
            }

            let mut num_s = String::new();
            let mut has_dot = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_ascii_digit() {
                    num_s.push(ch);
                    i += 1;
                } else if ch == '_' {
                    i += 1;
                } else if !has_dot && is_char_thousand_space(&chars, i) {
                    // Absorb space-separated thousand group (`250 000` -> `250000`)
                    num_s.push(chars[i + 1]);
                    num_s.push(chars[i + 2]);
                    num_s.push(chars[i + 3]);
                    i += 4;
                } else if (ch == '.' || (ch == ',' && is_decimal_comma(&chars, i, in_fn_call)))
                    && !has_dot
                    && i + 1 < chars.len()
                    && chars[i + 1].is_ascii_digit()
                {
                    has_dot = true;
                    num_s.push('.');
                    i += 1;
                } else if (ch == 'e' || ch == 'E')
                    && i + 1 < chars.len()
                    && (chars[i + 1].is_ascii_digit()
                        || ((chars[i + 1] == '+' || chars[i + 1] == '-')
                            && i + 2 < chars.len()
                            && chars[i + 2].is_ascii_digit()))
                {
                    num_s.push('e');
                    i += 1;
                    if chars[i] == '+' || chars[i] == '-' {
                        num_s.push(chars[i]);
                        i += 1;
                    }
                } else {
                    break;
                }
            }

            let mut val: f64 = num_s.parse().map_err(|_| format!("Invalid number: {}", num_s))?;
            if i < chars.len() {
                let next_is_alpha = i + 1 < chars.len() && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '_');
                if !next_is_alpha {
                    match chars[i] {
                        'k' | 'K' | 'к' | 'К' => {
                            val *= 1e3;
                            i += 1;
                        }
                        'm' | 'M' | 'м' | 'М' => {
                            val *= 1e6;
                            i += 1;
                        }
                        'b' | 'B' => {
                            val *= 1e9;
                            i += 1;
                        }
                        _ => {}
                    }
                }
            }

            tokens.push(LexToken::Num(val));
            continue;
        }

        if c == '#' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            i += 1;
            let mut digits = String::from("#");
            while i < chars.len() && chars[i].is_ascii_digit() {
                digits.push(chars[i]);
                i += 1;
            }
            tokens.push(LexToken::Ident(digits));
            continue;
        }

        if c.is_alphabetic() || c == '_' || c == 'π' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == 'π') {
                ident.push(chars[i]);
                i += 1;
            }
            let lower = ident.to_lowercase();
            match lower.as_str() {
                "of" | "от" | "из" => tokens.push(LexToken::Of),
                "mod" => tokens.push(LexToken::Ident("__mod__".to_string())),
                "deg" | "градус" | "град" => tokens.push(LexToken::Deg),
                _ => tokens.push(LexToken::Ident(lower)),
            }
            continue;
        }

        match c {
            '+' => {
                tokens.push(LexToken::Plus);
                i += 1;
            }
            '-' | '−' => {
                tokens.push(LexToken::Minus);
                i += 1;
            }
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    tokens.push(LexToken::Caret);
                    i += 2;
                } else {
                    tokens.push(LexToken::Star);
                    i += 1;
                }
            }
            '×' => {
                tokens.push(LexToken::Star);
                i += 1;
            }
            '/' => {
                if i + 1 < chars.len() && chars[i + 1] == '/' {
                    tokens.push(LexToken::IntDiv);
                    i += 2;
                } else {
                    tokens.push(LexToken::Slash);
                    i += 1;
                }
            }
            '÷' => {
                tokens.push(LexToken::Slash);
                i += 1;
            }
            '%' => {
                tokens.push(LexToken::Percent);
                i += 1;
            }
            '^' => {
                tokens.push(LexToken::Caret);
                i += 1;
            }
            '!' => {
                tokens.push(LexToken::Bang);
                i += 1;
            }
            '°' => {
                tokens.push(LexToken::Deg);
                i += 1;
            }
            '(' => {
                let prev_is_fn = matches!(tokens.last(), Some(LexToken::Ident(s)) if s != "__mod__");
                fn_paren_stack.push(prev_is_fn);
                tokens.push(LexToken::LParen);
                i += 1;
            }
            ')' => {
                fn_paren_stack.pop();
                tokens.push(LexToken::RParen);
                i += 1;
            }
            ',' | ';' => {
                tokens.push(LexToken::Comma);
                i += 1;
            }
            _ => {
                return Err(format!("Unexpected character: '{}'", c));
            }
        }
    }

    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<LexToken>,
    pos: usize,
    vars: &'a HashMap<String, f64>,
    line_values: &'a [Option<f64>],
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&LexToken> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<LexToken> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_add_sub(&mut self) -> Result<Val, String> {
        let mut left = self.parse_mul_div()?;
        while let Some(tok) = self.peek() {
            match tok {
                LexToken::Plus => {
                    self.next();
                    let right = self.parse_mul_div()?;
                    let v = match (left, right) {
                        (Val::Plain(a), Val::Percent(p)) => a * (1.0 + p),
                        (a, b) => a.as_f64() + b.as_f64(),
                    };
                    left = Val::Plain(v);
                }
                LexToken::Minus => {
                    self.next();
                    let right = self.parse_mul_div()?;
                    let v = match (left, right) {
                        (Val::Plain(a), Val::Percent(p)) => a * (1.0 - p),
                        (a, b) => a.as_f64() - b.as_f64(),
                    };
                    left = Val::Plain(v);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn can_start_implicit_mul(&self) -> bool {
        match self.peek() {
            Some(LexToken::LParen) => true,
            Some(LexToken::Ident(s)) if s != "__mod__" => true,
            _ => false,
        }
    }

    fn parse_mul_div(&mut self) -> Result<Val, String> {
        let mut left = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(LexToken::Star) | Some(LexToken::Of) => {
                    self.next();
                    let right = self.parse_unary()?;
                    left = Val::Plain(left.as_f64() * right.as_f64());
                }
                Some(LexToken::Slash) => {
                    self.next();
                    let right = self.parse_unary()?;
                    left = Val::Plain(left.as_f64() / right.as_f64());
                }
                Some(LexToken::IntDiv) => {
                    self.next();
                    let right = self.parse_unary()?;
                    left = Val::Plain((left.as_f64() / right.as_f64()).floor());
                }
                Some(LexToken::Ident(s)) if s == "__mod__" => {
                    self.next();
                    let right = self.parse_unary()?;
                    left = Val::Plain(left.as_f64() % right.as_f64());
                }
                _ if self.can_start_implicit_mul() => {
                    let right = self.parse_unary()?;
                    left = Val::Plain(left.as_f64() * right.as_f64());
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Val, String> {
        match self.peek() {
            Some(LexToken::Plus) => {
                self.next();
                self.parse_unary()
            }
            Some(LexToken::Minus) => {
                self.next();
                let v = self.parse_unary()?;
                Ok(match v {
                    Val::Plain(x) => Val::Plain(-x),
                    Val::Percent(p) => Val::Percent(-p),
                })
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<Val, String> {
        let base = self.parse_postfix()?;
        if matches!(self.peek(), Some(LexToken::Caret)) {
            self.next();
            let exp = self.parse_unary()?;
            Ok(Val::Plain(base.as_f64().powf(exp.as_f64())))
        } else {
            Ok(base)
        }
    }

    fn parse_postfix(&mut self) -> Result<Val, String> {
        let mut val = self.parse_primary()?;
        loop {
            match self.peek() {
                Some(LexToken::Percent) => {
                    self.next();
                    if matches!(
                        self.peek(),
                        Some(LexToken::Num(_)) | Some(LexToken::LParen) | Some(LexToken::Ident(_))
                    ) {
                        let rhs = self.parse_unary()?;
                        val = Val::Plain(val.as_f64() % rhs.as_f64());
                    } else {
                        val = Val::Percent(val.as_f64() / 100.0);
                    }
                }
                Some(LexToken::Bang) => {
                    self.next();
                    let n = val.as_f64();
                    if n < 0.0 || (n - n.round()).abs() > 1e-9 || n > 170.0 {
                        return Err("Invalid factorial".to_string());
                    }
                    let mut acc = 1.0;
                    for k in 2..=(n.round() as u64) {
                        acc *= k as f64;
                    }
                    val = Val::Plain(acc);
                }
                Some(LexToken::Deg) => {
                    self.next();
                    val = Val::Plain(val.as_f64() * PI / 180.0);
                }
                _ => break,
            }
        }
        Ok(val)
    }

    fn parse_primary(&mut self) -> Result<Val, String> {
        match self.next() {
            Some(LexToken::Num(n)) => Ok(Val::Plain(n)),
            Some(LexToken::LParen) => {
                let v = self.parse_add_sub()?;
                if !matches!(self.next(), Some(LexToken::RParen)) {
                    return Err("Unclosed '('".to_string());
                }
                Ok(v)
            }
            Some(LexToken::Ident(name)) => {
                if matches!(self.peek(), Some(LexToken::LParen)) {
                    self.next();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(LexToken::RParen)) {
                        loop {
                            args.push(self.parse_add_sub()?.as_f64());
                            if matches!(self.peek(), Some(LexToken::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    if !matches!(self.next(), Some(LexToken::RParen)) {
                        return Err("Unclosed function '('".to_string());
                    }
                    let res = eval_function(&name, &args, self.line_values)?;
                    return Ok(Val::Plain(res));
                }

                match name.as_str() {
                    "pi" | "π" | "пи" => Ok(Val::Plain(PI)),
                    "e" => Ok(Val::Plain(E)),
                    "tau" => Ok(Val::Plain(TAU)),
                    "phi" => Ok(Val::Plain(1.618033988749895)),
                    "sum" | "total" | "сумма" | "итого" => {
                        let vals = preceding_block_values(self.line_values);
                        Ok(Val::Plain(vals.iter().sum()))
                    }
                    "avg" | "mean" | "среднее" | "срзнач" => {
                        let vals = preceding_block_values(self.line_values);
                        if vals.is_empty() {
                            Ok(Val::Plain(0.0))
                        } else {
                            Ok(Val::Plain(vals.iter().sum::<f64>() / vals.len() as f64))
                        }
                    }
                    "prev" | "ans" | "_" | "пред" | "ответ" => {
                        let prev = self
                            .line_values
                            .iter()
                            .rev()
                            .flatten()
                            .next()
                            .copied()
                            .ok_or_else(|| "No previous line value".to_string())?;
                        Ok(Val::Plain(prev))
                    }
                    s if s.starts_with('#') || s.starts_with("line") || s.starts_with("строка") => {
                        let num_part = s
                            .trim_start_matches('#')
                            .trim_start_matches("line")
                            .trim_start_matches("строка");
                        if let Ok(idx1) = num_part.parse::<usize>() {
                            if idx1 >= 1 && idx1 <= self.line_values.len() {
                                if let Some(v) = self.line_values[idx1 - 1] {
                                    return Ok(Val::Plain(v));
                                }
                            }
                        }
                        Err(format!("Line {} has no value", s))
                    }
                    _ => {
                        if let Some(&v) = self.vars.get(&name) {
                            Ok(Val::Plain(v))
                        } else {
                            Err(format!("Unknown variable '{}'", name))
                        }
                    }
                }
            }
            other => Err(format!("Unexpected token: {:?}", other)),
        }
    }
}

fn eval_function(name: &str, args: &[f64], line_values: &[Option<f64>]) -> Result<f64, String> {
    let req1 = || -> Result<f64, String> {
        if args.len() != 1 {
            Err(format!("{}() takes 1 argument", name))
        } else {
            Ok(args[0])
        }
    };

    match name {
        "sqrt" | "корень" => Ok(req1()?.sqrt()),
        "cbrt" => Ok(req1()?.cbrt()),
        "abs" | "модуль" => Ok(req1()?.abs()),
        "round" | "окр" => {
            if args.len() == 1 {
                Ok(args[0].round())
            } else if args.len() == 2 {
                let factor = 10f64.powi(args[1] as i32);
                Ok((args[0] * factor).round() / factor)
            } else {
                Err("round(x, [digits])".to_string())
            }
        }
        "floor" => Ok(req1()?.floor()),
        "ceil" => Ok(req1()?.ceil()),
        "trunc" => Ok(req1()?.trunc()),
        "sin" => Ok(req1()?.sin()),
        "cos" => Ok(req1()?.cos()),
        "tan" => Ok(req1()?.tan()),
        "asin" => Ok(req1()?.asin()),
        "acos" => Ok(req1()?.acos()),
        "atan" => Ok(req1()?.atan()),
        "sinh" => Ok(req1()?.sinh()),
        "cosh" => Ok(req1()?.cosh()),
        "tanh" => Ok(req1()?.tanh()),
        "ln" => Ok(req1()?.ln()),
        "log" | "log10" => {
            if args.len() == 1 {
                Ok(args[0].log10())
            } else if args.len() == 2 {
                Ok(args[0].log(args[1]))
            } else {
                Err("log(x, [base])".to_string())
            }
        }
        "log2" => Ok(req1()?.log2()),
        "exp" => Ok(req1()?.exp()),
        "deg" => Ok(req1()?.to_degrees()),
        "rad" => Ok(req1()?.to_radians()),
        "pow" | "степень" => {
            if args.len() != 2 {
                Err("pow(base, exp) takes 2 arguments".to_string())
            } else {
                Ok(args[0].powf(args[1]))
            }
        }
        "min" | "мин" => args
            .iter()
            .copied()
            .reduce(f64::min)
            .ok_or_else(|| "min() requires arguments".to_string()),
        "max" | "макс" => args
            .iter()
            .copied()
            .reduce(f64::max)
            .ok_or_else(|| "max() requires arguments".to_string()),
        "sum" | "total" | "сумма" | "итого" => {
            if args.is_empty() {
                Ok(preceding_block_values(line_values).iter().sum())
            } else {
                Ok(args.iter().sum())
            }
        }
        "avg" | "mean" | "среднее" | "срзнач" => {
            let list: Vec<f64> = if args.is_empty() {
                preceding_block_values(line_values)
            } else {
                args.to_vec()
            };
            if list.is_empty() {
                Ok(0.0)
            } else {
                Ok(list.iter().sum::<f64>() / list.len() as f64)
            }
        }
        "fact" => {
            let n = req1()?;
            if n < 0.0 || n > 170.0 {
                return Err("Invalid factorial".to_string());
            }
            let mut acc = 1.0;
            for k in 2..=(n.round() as u64) {
                acc *= k as f64;
            }
            Ok(acc)
        }
        _ => Err(format!("Unknown function '{}'", name)),
    }
}

fn eval_expr(
    input: &str,
    vars: &HashMap<String, f64>,
    line_values: &[Option<f64>],
) -> Result<f64, String> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err("Empty expression".to_string());
    }
    let mut parser = Parser {
        tokens,
        pos: 0,
        vars,
        line_values,
    };
    let val = parser.parse_add_sub()?;
    if parser.pos < parser.tokens.len() {
        return Err(format!("Unexpected trailing token: {:?}", parser.tokens[parser.pos]));
    }
    Ok(val.as_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_lines(doc: &str) -> Vec<Option<String>> {
        evaluate_document(doc)
            .into_iter()
            .map(|l| if l.formatted.is_empty() { None } else { Some(l.formatted) })
            .collect()
    }

    #[test]
    fn test_basic_arithmetic_and_precedence() {
        let res = eval_lines("2 + 3 * 4\n(2 + 3) * 4\n2^10\n2**10\n10 // 3\n10 % 3\n5!");
        assert_eq!(
            res,
            vec![
                Some("14".into()),
                Some("20".into()),
                Some("1 024".into()),
                Some("1 024".into()),
                Some("3".into()),
                Some("1".into()),
                Some("120".into()),
            ]
        );
    }

    #[test]
    fn test_pasted_space_grouped_numbers() {
        // Copying "250 000" from the answer column and pasting it back must evaluate as 250000!
        let res = eval_lines("250 000 + 65 000\n1 250 000.5 - 250 000");
        assert_eq!(
            res,
            vec![Some("315 000".into()), Some("1 000 000.5".into())]
        );
    }

    #[test]
    fn test_percentages_and_russian_ot() {
        let res = eval_lines(
            "1250 + 20%\n1250 - 20%\n20% of 1250\n6% от 250k\n120 / (1 + 20%)",
        );
        assert_eq!(
            res,
            vec![
                Some("1 500".into()),
                Some("1 000".into()),
                Some("250".into()),
                Some("15 000".into()),
                Some("100".into()),
            ]
        );
    }

    #[test]
    fn test_variables_labels_and_compound_assign() {
        let doc = "\
// Бюджет
доход = 250k
аренда: 65000
еда: 12500 + 4300
доход += 50к
остаток = доход - аренда - еда";
        let res = eval_lines(doc);
        assert_eq!(
            res,
            vec![
                None,
                Some("250 000".into()),
                Some("65 000".into()),
                Some("16 800".into()),
                Some("300 000".into()),
                Some("218 200".into()),
            ]
        );
    }

    #[test]
    fn test_functions_and_comma_disambiguation() {
        // Both pow(2,10) without spaces, round(3,1415926 * 2, 3) with decimal comma inside fn args,
        // and decimal comma 3,14 * 2 outside fn args must work!
        let res = eval_lines(
            "pow(2,10)\nround(3,1415926 * 2, 3)\nmin(15, 8, 42)\nmax(15, 8, 42)\n3,14 * 2\nsin(45 deg)^2 + cos(45 deg)^2",
        );
        assert_eq!(
            res,
            vec![
                Some("1 024".into()),
                Some("6.283".into()),
                Some("8".into()),
                Some("42".into()),
                Some("6.28".into()),
                Some("1".into()),
            ]
        );
    }

    #[test]
    fn test_line_references_and_aggregations() {
        let doc = "\
100
200
300
sum
avg
#1 + #2
prev * 2";
        let res = eval_lines(doc);
        assert_eq!(
            res,
            vec![
                Some("100".into()),
                Some("200".into()),
                Some("300".into()),
                Some("600".into()),
                Some("300".into()), // avg of 100, 200, 300, 600 = 1200 / 4 = 300
                Some("300".into()), // #1 + #2 = 100 + 200
                Some("600".into()), // prev * 2 = 300 * 2
            ]
        );
    }

    #[test]
    fn test_hex_binary_and_implicit_mul() {
        let res = eval_lines("0xff + 1\n0b1010 * 3\n2(3 + 4)\n(1 + 2)(3 + 4)");
        assert_eq!(
            res,
            vec![
                Some("256".into()),
                Some("30".into()),
                Some("14".into()),
                Some("21".into()),
            ]
        );
    }

    #[test]
    fn test_russian_functions_and_inline_comments() {
        let doc = "\
корень(144) // длина стороны
окр(пи, 4) # константа
мин(10, 25, 3) + макс(4, 9)
степень(2, 8)
20% из 500
пред + строка1";
        let res = eval_lines(doc);
        assert_eq!(
            res,
            vec![
                Some("12".into()),
                Some("3.1416".into()),
                Some("12".into()),
                Some("256".into()),
                Some("100".into()),
                Some("112".into()),
            ]
        );
    }

    #[test]
    fn test_edge_cases_infinity_nan_and_blocks() {
        let doc = "\
1 / 0
-1 / 0
0 / 0
2^-3
-3^2
(-3)^2

10
20
сумма";
        let res = eval_lines(doc);
        assert_eq!(
            res,
            vec![
                Some("∞".into()),
                Some("-∞".into()),
                Some("NaN".into()),
                Some("0.125".into()),
                Some("-9".into()),
                Some("9".into()),
                None,
                Some("10".into()),
                Some("20".into()),
                Some("30".into()),
            ]
        );
    }
}
