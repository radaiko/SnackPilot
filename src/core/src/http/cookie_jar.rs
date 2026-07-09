//! Ventopay's app-owned cookie jar. Exact v1 semantics: substring before first ';',
//! split on first '=', ignore attributes, never expire, overwrite preserving insertion
//! position, emit "n1=v1; n2=v2" in insertion order, no header when empty
//! (02-ventopay-scraping §2.2; v1: ventopayClient.ts:31-58).

/// Insertion-ordered name→value store.
#[derive(Debug, Default, Clone)]
pub struct CookieJar {
    entries: Vec<(String, String)>, // insertion order preserved
}

impl CookieJar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn capture(&mut self, set_cookie_values: &[String]) {
        for raw in set_cookie_values {
            let name_value = raw.split(';').next().unwrap_or("");
            let eq = match name_value.find('=') {
                Some(i) if i > 0 => i, // '=' absent or at index 0 → ignore
                _ => continue,
            };
            let name = name_value[..eq].trim().to_string();
            let value = name_value[eq + 1..].trim().to_string();
            match self.entries.iter_mut().find(|(n, _)| *n == name) {
                Some(slot) => slot.1 = value, // overwrite in place
                None => self.entries.push((name, value)),
            }
        }
    }

    pub fn header(&self) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        Some(
            self.entries
                .iter()
                .map(|(n, v)| format!("{n}={v}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_name_value_ignoring_attributes() {
        // take substring before first ';', split on first '=' (02 §2.2).
        let mut jar = CookieJar::new();
        jar.capture(&["ASP.NET_SessionId=abc123; path=/; HttpOnly".to_string()]);
        assert_eq!(jar.header().as_deref(), Some("ASP.NET_SessionId=abc123"));
    }

    #[test]
    fn overwrite_preserves_insertion_position() {
        let mut jar = CookieJar::new();
        jar.capture(&["a=1".into()]);
        jar.capture(&["b=2".into()]);
        jar.capture(&["a=3".into()]); // overwrite a, keep position
        assert_eq!(jar.header().as_deref(), Some("a=3; b=2"));
    }

    #[test]
    fn ignores_malformed_and_empty() {
        let mut jar = CookieJar::new();
        jar.capture(&["=novalue".into(), "noequals".into()]);
        assert_eq!(jar.header(), None); // '=' at index 0 or absent → ignored
    }

    #[test]
    fn no_header_when_empty() {
        assert_eq!(CookieJar::new().header(), None);
    }
}
