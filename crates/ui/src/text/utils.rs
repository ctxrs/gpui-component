const NUMBERED_PREFIXES_1: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERED_PREFIXES_2: &str = "abcdefghijklmnopqrstuvwxyz";

const BULLETS: [&str; 5] = ["▪", "•", "◦", "‣", "⁃"];

/// Returns the prefix for a list item.
pub(super) fn list_item_prefix(ix: usize, ordered: bool, depth: usize) -> String {
    if ordered {
        if depth == 0 {
            return format!("{}. ", ix + 1);
        }

        if depth == 1 {
            return format!(
                "{}. ",
                NUMBERED_PREFIXES_1
                    .chars()
                    .nth(ix % NUMBERED_PREFIXES_1.len())
                    .unwrap()
            );
        } else {
            return format!(
                "{}. ",
                NUMBERED_PREFIXES_2
                    .chars()
                    .nth(ix % NUMBERED_PREFIXES_2.len())
                    .unwrap()
            );
        }
    } else {
        let depth = depth.min(BULLETS.len() - 1);
        let bullet = BULLETS[depth];
        return format!("{} ", bullet);
    }
}


#[derive(Debug, Clone)]
pub(crate) struct FileRef {
    pub(crate) path: String,
    pub(crate) line: Option<u32>,
    pub(crate) col: Option<u32>,
}

pub(crate) fn split_whitespace_token_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut start: Option<usize> = None;
    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(begin) = start.take() {
                ranges.push(begin..idx);
            }
        } else if start.is_none() {
            start = Some(idx);
        }
    }
    if let Some(begin) = start {
        ranges.push(begin..text.len());
    }
    ranges
}

fn is_windows_drive(path: &str) -> bool {
    let bytes = path.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn has_explicit_path_cue(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if path == "." || path == ".." || path == "~" {
        return true;
    }
    if path.contains("://") {
        return false;
    }
    if path.starts_with("./") || path.starts_with("../") || path.starts_with("~/") {
        return true;
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return true;
    }
    if is_windows_drive(path) {
        return true;
    }
    path.contains('/') || path.contains('\\')
}

fn parse_digits(value: &str) -> Option<u32> {
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    value.parse::<u32>().ok().filter(|v| *v > 0)
}

fn parse_line_col_suffix(raw: &str) -> Option<FileRef> {
    if let Some(idx) = raw.rfind("#L") {
        let path = &raw[..idx];
        if !has_explicit_path_cue(path) {
            return None;
        }
        let rest = &raw[idx + 2..];
        let (line_part, col_part) = match rest.split_once('C') {
            Some((line, col)) => (line, Some(col)),
            None => (rest, None),
        };
        let line = parse_digits(line_part)?;
        let col = col_part.and_then(parse_digits);
        return Some(FileRef {
            path: path.to_string(),
            line: Some(line),
            col,
        });
    }

    if let Some((before_last, last_part)) = raw.rsplit_once(':') {
        let last_num = match parse_digits(last_part) {
            Some(val) => val,
            None => return None,
        };
        if let Some((path, line_part)) = before_last.rsplit_once(':') {
            if let Some(line) = parse_digits(line_part) {
                if !has_explicit_path_cue(path) {
                    return None;
                }
                return Some(FileRef {
                    path: path.to_string(),
                    line: Some(line),
                    col: Some(last_num),
                });
            }
        }
        if !has_explicit_path_cue(before_last) {
            return None;
        }
        return Some(FileRef {
            path: before_last.to_string(),
            line: Some(last_num),
            col: None,
        });
    }

    None
}

pub(crate) fn parse_file_ref_token(raw: &str) -> Option<FileRef> {
    if raw.is_empty() {
        return None;
    }
    if raw.contains("://") {
        return None;
    }
    if let Some(file) = parse_line_col_suffix(raw) {
        return Some(file);
    }
    if !has_explicit_path_cue(raw) {
        return None;
    }
    Some(FileRef {
        path: raw.to_string(),
        line: None,
        col: None,
    })
}

pub(crate) fn parse_url_token(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return None;
    }
    Some(raw.to_string())
}

pub(crate) fn is_absolute_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if path == "~" || path.starts_with("~/") {
        return true;
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return true;
    }
    is_windows_drive(path)
}

pub(crate) fn encode_uri_component(value: &str) -> String {
    let mut out = String::new();
    for b in value.as_bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => out.push(*b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::text::utils::list_item_prefix;

    #[test]
    fn test_list_item_prefix() {
        assert_eq!(list_item_prefix(0, true, 0), "1. ");
        assert_eq!(list_item_prefix(1, true, 0), "2. ");
        assert_eq!(list_item_prefix(2, true, 0), "3. ");
        assert_eq!(list_item_prefix(10, true, 0), "11. ");
        assert_eq!(list_item_prefix(0, true, 1), "A. ");
        assert_eq!(list_item_prefix(1, true, 1), "B. ");
        assert_eq!(list_item_prefix(2, true, 1), "C. ");
        assert_eq!(list_item_prefix(0, true, 2), "a. ");
        assert_eq!(list_item_prefix(1, true, 2), "b. ");
        assert_eq!(list_item_prefix(6, true, 2), "g. ");
        assert_eq!(list_item_prefix(0, true, 1), "A. ");
        assert_eq!(list_item_prefix(0, true, 2), "a. ");
        assert_eq!(list_item_prefix(0, false, 0), "▪ ");
        assert_eq!(list_item_prefix(0, false, 1), "• ");
        assert_eq!(list_item_prefix(0, false, 2), "◦ ");
        assert_eq!(list_item_prefix(0, false, 3), "‣ ");
        assert_eq!(list_item_prefix(0, false, 4), "⁃ ");
    }
}
