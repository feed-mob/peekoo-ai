pub fn build_url_with_query(base: &str, params: &[(&str, &str)]) -> String {
    let mut url = String::with_capacity(base.len() + 128);
    url.push_str(base);
    url.push('?');

    for (index, (key, value)) in params.iter().enumerate() {
        if index > 0 {
            url.push('&');
        }
        url.push_str(&percent_encode_component(key));
        url.push('=');
        url.push_str(&percent_encode_component(value));
    }
    url
}

pub fn parse_query_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            let key = percent_decode_component(key.trim())?;
            let value = percent_decode_component(value.trim())?;
            Some((key, value))
        })
        .collect()
}

fn percent_encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push_str("%20"),
            other => {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("%{other:02X}"));
            }
        }
    }
    out
}

fn percent_decode_component(value: &str) -> Option<String> {
    if !value.as_bytes().contains(&b'%') && !value.as_bytes().contains(&b'+') {
        return Some(value.to_string());
    }

    let mut out = Vec::with_capacity(value.len());
    let mut bytes = value.as_bytes().iter().copied();
    while let Some(byte) = bytes.next() {
        match byte {
            b'+' => out.push(b' '),
            b'%' => {
                let hi = bytes.next()?;
                let lo = bytes.next()?;
                let hex_bytes = [hi, lo];
                let hex = std::str::from_utf8(&hex_bytes).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
            }
            other => out.push(other),
        }
    }

    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query_pairs_decodes_values() {
        let pairs = parse_query_pairs("code=abc123&state=hello%20world");
        assert!(pairs.iter().any(|(k, v)| k == "code" && v == "abc123"));
        assert!(
            pairs
                .iter()
                .any(|(k, v)| k == "state" && v == "hello world")
        );
    }
}
