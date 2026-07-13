use crate::{
    YfClient, YfError,
    core::{
        client::{RetryConfig, normalize_symbol},
        net,
    },
};
use paft::domain::Isin;
use serde::Deserialize;
use serde_json::{Number, Value};

type InsiderSuggestArgs = (Value, Vec<String>, Vec<Vec<String>>, Value, Value);

#[derive(Deserialize)]
struct JsonSuggestRow {
    #[serde(alias = "Value", alias = "value")]
    value: Option<String>,
    #[serde(alias = "Symbol", alias = "symbol")]
    symbol: Option<String>,
    #[serde(alias = "Isin", alias = "isin", alias = "ISIN")]
    isin: Option<String>,
}

#[derive(Debug)]
struct InsiderSuggestResponse {
    rows: Vec<InsiderSuggestRow>,
}

#[derive(Debug, Default)]
struct InsiderSuggestRow {
    keywords: Option<String>,
    ids: Option<String>,
}

pub(super) async fn fetch_isin(
    client: &YfClient,
    symbol: &str,
    retry_override: Option<&RetryConfig>,
) -> Result<Option<String>, YfError> {
    let symbol = normalize_symbol(symbol)?;
    let body = fetch_isin_body(client, &symbol, retry_override).await?;

    let input_norm = normalize_sym(&symbol);

    if let Some(isin) = parse_business_insider_suggest(&body, &input_norm) {
        return Ok(Some(isin));
    }

    if let Some(isin) = parse_json_suggest_rows(&body, &input_norm) {
        return Ok(Some(isin));
    }

    crate::core::logging::trace_debug!("no matching ISIN found in any response shape");
    Ok(None)
}

async fn fetch_isin_body(
    client: &YfClient,
    symbol: &str,
    retry_override: Option<&RetryConfig>,
) -> Result<String, YfError> {
    let symbol = normalize_symbol(symbol)?;
    let mut url = client.base_insider_search().clone();
    url.query_pairs_mut()
        .append_pair("max_results", "5")
        .append_pair("query", &symbol);

    let req = client.http().get(url.clone());
    let resp = client.send_with_retry(req, retry_override).await?;

    if !resp.status().is_success() {
        return Err(net::status_error(resp.status(), &url));
    }

    net::get_text(resp, "isin_search", &symbol, "json")
        .await
        .map_err(YfError::from)
}

fn parse_business_insider_suggest(body: &str, input_norm: &str) -> Option<String> {
    let response = parse_business_insider_suggest_response(body)?;
    for row in &response.rows {
        if let Some(isin) = row.isin_for_symbol(input_norm) {
            crate::core::logging::trace_debug!(
                isin = %isin,
                "ISIN extracted from Business Insider suggest response"
            );
            return Some(isin);
        }
    }
    None
}

fn parse_business_insider_suggest_response(body: &str) -> Option<InsiderSuggestResponse> {
    let args = business_insider_callback_args(body)?;
    let (_, columns, rows, _, _): InsiderSuggestArgs =
        serde_json::from_value(Value::Array(args)).ok()?;
    Some(InsiderSuggestResponse::from_parts(&columns, rows))
}

fn business_insider_callback_args(body: &str) -> Option<Vec<Value>> {
    JsDataParser::new(body).parse_mm_suggest_deliver_args()
}

fn parse_json_suggest_rows(body: &str, input_norm: &str) -> Option<String> {
    let rows = serde_json::from_str::<Vec<JsonSuggestRow>>(body).ok()?;
    let allow_unscoped = rows.len() == 1;
    for row in &rows {
        if let Some(isin) = row.isin.as_deref().and_then(validated_isin)
            && symbol_scope_matches(row.symbol.as_deref(), input_norm, allow_unscoped)
        {
            return Some(isin);
        }
        if let Some(value) = row.value.as_deref()
            && let Some(isin) = pick_from_pipe_value(value, input_norm)
        {
            return Some(isin);
        }
    }
    None
}

impl InsiderSuggestResponse {
    fn from_parts(columns: &[String], raw_rows: Vec<Vec<String>>) -> Self {
        let rows = raw_rows
            .into_iter()
            .map(|cells| InsiderSuggestRow::from_cells(columns, &cells))
            .collect();
        Self { rows }
    }
}

impl InsiderSuggestRow {
    fn from_cells(columns: &[String], cells: &[String]) -> Self {
        let mut row = Self::default();
        for (column, cell) in columns.iter().zip(cells) {
            match column.as_str() {
                "Keywords" => row.keywords = Some(cell.clone()),
                "IDs" => row.ids = Some(cell.clone()),
                _ => {}
            }
        }
        row
    }

    fn isin_for_symbol(&self, target_norm: &str) -> Option<String> {
        let symbol = self.symbol_hint()?;
        if normalize_sym(symbol) != target_norm {
            return None;
        }

        self.keywords
            .as_deref()
            .into_iter()
            .flat_map(pipe_parts)
            .find_map(validated_isin)
    }

    fn symbol_hint(&self) -> Option<&str> {
        self.keywords
            .as_deref()
            .and_then(first_pipe_part)
            .or_else(|| self.ids.as_deref().and_then(second_pipe_part))
    }
}

fn normalize_sym(s: &str) -> String {
    s.trim()
        .chars()
        .map(|ch| match ch {
            '-' | ':' => '.',
            ch if ch.is_ascii_whitespace() => '.',
            ch => ch.to_ascii_lowercase(),
        })
        .collect()
}

fn symbol_scope_matches(symbol: Option<&str>, target_norm: &str, allow_unscoped: bool) -> bool {
    symbol
        .filter(|symbol| !symbol.trim().is_empty())
        .map_or(allow_unscoped, |symbol| {
            normalize_sym(symbol) == target_norm
        })
}

fn validated_isin(candidate: &str) -> Option<String> {
    Isin::new(candidate).ok().map(String::from)
}

fn pipe_parts(value: &str) -> impl Iterator<Item = &str> {
    value
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
}

fn first_pipe_part(value: &str) -> Option<&str> {
    pipe_parts(value).next()
}

fn second_pipe_part(value: &str) -> Option<&str> {
    pipe_parts(value).nth(1)
}

fn pick_from_pipe_value(value: &str, target_norm: &str) -> Option<String> {
    let mut parts = pipe_parts(value);
    let first = parts.next()?;
    if normalize_sym(first) != target_norm {
        return None;
    }

    parts.find_map(validated_isin)
}

struct JsDataParser<'a> {
    source: &'a str,
    pos: usize,
}

impl<'a> JsDataParser<'a> {
    const fn new(source: &'a str) -> Self {
        Self { source, pos: 0 }
    }

    fn parse_mm_suggest_deliver_args(mut self) -> Option<Vec<Value>> {
        self.skip_trivia();
        if self.parse_identifier()? != "mmSuggestDeliver" {
            return None;
        }
        self.skip_trivia();
        self.expect_byte(b'(')?;
        let args = self.parse_values_until(b')')?;
        self.skip_trivia();
        self.consume_byte(b';');
        self.skip_trivia();

        self.is_eof().then_some(args)
    }

    fn parse_value(&mut self) -> Option<Value> {
        self.skip_trivia();
        match self.peek_byte()? {
            b'"' | b'\'' => self.parse_string().map(Value::String),
            b'[' => self.parse_array_literal(),
            b'(' => self.parse_parenthesized(),
            b'0'..=b'9' => self.parse_number(),
            byte if is_identifier_start(byte) => self.parse_keyword_or_constructor(),
            _ => None,
        }
    }

    fn parse_keyword_or_constructor(&mut self) -> Option<Value> {
        match self.parse_identifier()? {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            "null" => Some(Value::Null),
            "new" => self.parse_array_constructor(),
            _ => None,
        }
    }

    fn parse_array_constructor(&mut self) -> Option<Value> {
        self.skip_trivia();
        if self.parse_identifier()? != "Array" {
            return None;
        }
        self.skip_trivia();
        self.expect_byte(b'(')?;
        self.parse_values_until(b')').map(Value::Array)
    }

    fn parse_array_literal(&mut self) -> Option<Value> {
        self.expect_byte(b'[')?;
        self.parse_values_until(b']').map(Value::Array)
    }

    fn parse_parenthesized(&mut self) -> Option<Value> {
        self.expect_byte(b'(')?;
        let value = self.parse_value()?;
        self.skip_trivia();
        self.expect_byte(b')')?;
        Some(value)
    }

    fn parse_values_until(&mut self, terminator: u8) -> Option<Vec<Value>> {
        let mut values = Vec::new();
        loop {
            self.skip_trivia();
            if self.consume_byte(terminator) {
                return Some(values);
            }

            values.push(self.parse_value()?);
            self.skip_trivia();

            if self.consume_byte(b',') {
                continue;
            }
            self.expect_byte(terminator)?;
            return Some(values);
        }
    }

    fn parse_number(&mut self) -> Option<Value> {
        let start = self.pos;
        self.consume_digits();

        if self.consume_byte(b'.') {
            self.consume_digits();
        }
        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            self.consume_digits();
        }

        let raw = &self.source[start..self.pos];
        raw.parse::<Number>().ok().map(Value::Number)
    }

    fn parse_string(&mut self) -> Option<String> {
        let quote = self.next_byte()?;
        let mut decoded = String::new();

        while !self.is_eof() {
            let byte = self.peek_byte()?;
            if byte == quote {
                self.pos += 1;
                return Some(decoded);
            }
            if byte == b'\\' {
                self.pos += 1;
                self.parse_escape(&mut decoded)?;
                continue;
            }
            if matches!(byte, b'\n' | b'\r') {
                return None;
            }
            decoded.push(self.next_char()?);
        }

        None
    }

    fn parse_escape(&mut self, decoded: &mut String) -> Option<()> {
        match self.next_char()? {
            'b' => decoded.push('\u{0008}'),
            'f' => decoded.push('\u{000c}'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            'v' => decoded.push('\u{000b}'),
            '0' if self.peek_byte().is_some_and(|byte| byte.is_ascii_digit()) => return None,
            '0' => decoded.push('\0'),
            '\n' | '\u{2028}' | '\u{2029}' => {}
            '\r' => {
                self.consume_byte(b'\n');
            }
            'x' => decoded.push(self.parse_hex_char(2)?),
            'u' => decoded.push(self.parse_unicode_escape()?),
            escaped => decoded.push(escaped),
        }

        Some(())
    }

    fn parse_hex_char(&mut self, digits: usize) -> Option<char> {
        char::from_u32(self.parse_hex_digits(digits)?)
    }

    fn parse_unicode_escape(&mut self) -> Option<char> {
        if self.consume_byte(b'{') {
            return self.parse_braced_unicode_escape();
        }

        let lead = self.parse_hex_digits(4)?;
        if is_high_surrogate(lead) {
            if !self.consume_str("\\u") {
                return None;
            }
            let trail = self.parse_hex_digits(4)?;
            if !is_low_surrogate(trail) {
                return None;
            }
            let scalar = 0x1_0000 + ((lead - 0xd800) << 10) + (trail - 0xdc00);
            return char::from_u32(scalar);
        }
        if is_low_surrogate(lead) {
            return None;
        }

        char::from_u32(lead)
    }

    fn parse_braced_unicode_escape(&mut self) -> Option<char> {
        let start = self.pos;
        let mut scalar = 0;

        while !self.consume_byte(b'}') {
            if self.pos == start + 6 {
                return None;
            }
            scalar = (scalar << 4) + hex_value(self.next_byte()?)?;
        }

        (self.pos > start + 1)
            .then_some(scalar)
            .and_then(char::from_u32)
    }

    fn parse_hex_digits(&mut self, digits: usize) -> Option<u32> {
        let mut value = 0;
        for _ in 0..digits {
            value = (value << 4) + hex_value(self.next_byte()?)?;
        }
        Some(value)
    }

    fn parse_identifier(&mut self) -> Option<&'a str> {
        let start = self.pos;
        if !is_identifier_start(self.peek_byte()?) {
            return None;
        }
        self.pos += 1;
        while self.peek_byte().is_some_and(is_identifier_continue) {
            self.pos += 1;
        }
        Some(&self.source[start..self.pos])
    }

    fn skip_trivia(&mut self) {
        loop {
            let start = self.pos;
            while self
                .peek_byte()
                .is_some_and(|byte| byte.is_ascii_whitespace())
            {
                self.pos += 1;
            }

            if self.consume_str("//") {
                while self
                    .peek_byte()
                    .is_some_and(|byte| !matches!(byte, b'\n' | b'\r'))
                {
                    self.pos += 1;
                }
            } else if self.consume_str("/*") {
                while !self.is_eof() && !self.consume_str("*/") {
                    let _ = self.next_char();
                }
            }

            if self.pos == start {
                break;
            }
        }
    }

    fn consume_digits(&mut self) {
        while self.peek_byte().is_some_and(|byte| byte.is_ascii_digit()) {
            self.pos += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Option<()> {
        self.consume_byte(expected).then_some(())
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn consume_str(&mut self, expected: &str) -> bool {
        if self.source[self.pos..].starts_with(expected) {
            self.pos += expected.len();
            true
        } else {
            false
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.pos += 1;
        Some(byte)
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.source[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.pos).copied()
    }

    const fn is_eof(&self) -> bool {
        self.pos == self.source.len()
    }
}

const fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
}

const fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

fn hex_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

fn is_high_surrogate(value: u32) -> bool {
    (0xd800..=0xdbff).contains(&value)
}

fn is_low_surrogate(value: u32) -> bool {
    (0xdc00..=0xdfff).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::JsDataParser;

    #[test]
    fn jsonp_parser_preserves_numeric_lexemes() {
        let args = JsDataParser::new(
            "mmSuggestDeliver(0.1234567890123456789012345678, new Array(), new Array(), 1.234567890123456789e+20, 0);",
        )
        .parse_mm_suggest_deliver_args()
        .expect("valid callback");

        assert_eq!(
            args[0].as_number().map(ToString::to_string).as_deref(),
            Some("0.1234567890123456789012345678")
        );
        assert_eq!(
            args[3].as_number().map(ToString::to_string).as_deref(),
            Some("1.234567890123456789e+20")
        );
    }
}
