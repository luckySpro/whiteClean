use std::path::Path;

/// Collapse copy and version markers so `报告.pdf` and `报告 副本.pdf` share a stem.
pub fn version_stem(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.to_string());
    let mut current = stem;
    loop {
        let next = strip_marker(&current);
        if next == current {
            break;
        }
        current = next;
    }
    current.trim().to_lowercase()
}

fn strip_marker(stem: &str) -> String {
    if let Some(stripped) = strip_copy_number(stem) {
        return stripped;
    }
    if let Some(stripped) = strip_suffix_word(stem, &["副本", "拷贝"]) {
        return stripped;
    }
    if let Some(stripped) = strip_suffix_word_ascii(stem, &["copy"]) {
        return stripped;
    }
    if let Some(stripped) = strip_version_tag(stem) {
        return stripped;
    }
    stem.to_string()
}

fn strip_copy_number(stem: &str) -> Option<String> {
    let bytes = stem.as_bytes();
    let end = bytes.len();
    let close = *bytes.last()?;
    let digits_end = if close == b')' {
        end - 1
    } else if stem.ends_with('）') {
        let open_at = stem.rfind('（')?;
        let digits = stem[open_at + '（'.len_utf8()..stem.len() - '）'.len_utf8()].trim();
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let rest = stem[..open_at].trim_end().trim_end_matches(['-', '_']).trim_end();
        if rest.is_empty() {
            return None;
        }
        return Some(rest.to_string());
    } else {
        return None;
    };
    let mut index = digits_end;
    let mut saw_digit = false;
    while index > 0 && bytes[index - 1].is_ascii_digit() {
        saw_digit = true;
        index -= 1;
    }
    if !saw_digit || bytes.get(index.checked_sub(1)?) != Some(&b'(') {
        return None;
    }
    let rest = stem[..index - 1].trim_end().trim_end_matches(['-', '_']).trim_end();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

fn strip_suffix_word(stem: &str, words: &[&str]) -> Option<String> {
    let trimmed = stem.trim_end();
    for word in words {
        if let Some(head) = trimmed.strip_suffix(word) {
            let rest = head.trim_end().trim_end_matches(['-', '_']).trim_end();
            if !rest.is_empty() && rest != trimmed {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn strip_suffix_word_ascii(stem: &str, words: &[&str]) -> Option<String> {
    let trimmed = stem.trim_end();
    let lower = trimmed.to_lowercase();
    for word in words {
        if let Some(head) = lower.strip_suffix(word) {
            if head.len() == lower.len() {
                continue;
            }
            let rest_len = trimmed.len() - (lower.len() - head.len());
            let rest = trimmed[..rest_len]
                .trim_end()
                .trim_end_matches(['-', '_'])
                .trim_end();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn strip_version_tag(stem: &str) -> Option<String> {
    let trimmed = stem.trim_end();
    let lower = trimmed.to_lowercase();
    let bytes = lower.as_bytes();
    let mut index = bytes.len();
    while index > 0 && bytes[index - 1].is_ascii_digit() {
        index -= 1;
    }
    if index == bytes.len() || index == 0 || bytes[index - 1] != b'v' {
        return None;
    }
    let sep_at = index - 1;
    if sep_at == 0 {
        return None;
    }
    let sep = bytes[sep_at - 1];
    if !matches!(sep, b' ' | b'-' | b'_') {
        return None;
    }
    let rest = trimmed[..sep_at - 1].trim_end();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}
