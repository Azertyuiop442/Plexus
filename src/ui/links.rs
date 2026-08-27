
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hit {

    Url(String),

    FilePath {
        path: String,
        line: Option<u32>,
        col: Option<u32>,
    },
}

const KNOWN_TLDS: &[&str] = &[
    "com", "org", "net", "edu", "gov", "io", "ai", "co", "me", "dev", "app", "xyz", "cloud",
    "tech", "site", "online", "sh", "gg", "gl", "fr", "uk", "de", "es", "eu", "us", "ca", "jp",
    "cn", "rs", "ts", "json", "md",
];

fn is_boundary(c: char) -> bool {
    c.is_whitespace() || c == '"' || c == '\'' || c == '`' || c == '<' || c == '>' || c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}' || c == '|'
}

pub fn link_at(line: &str, col: usize) -> Option<Hit> {
    if line.is_empty() {
        return None;
    }

    let chars: Vec<char> = line.chars().collect();
    if col >= chars.len() || is_boundary(chars[col]) {
        return None;
    }

    let mut start = col;
    while start > 0 && !is_boundary(chars[start - 1]) {
        start -= 1;
    }

    let mut end = col;
    while end + 1 < chars.len() && !is_boundary(chars[end + 1]) {
        end += 1;
    }

    let mut token: String = chars[start..=end].iter().collect();

    while token.ends_with('.') || token.ends_with(',') || token.ends_with(';') || token.ends_with(':') || token.ends_with('?') || token.ends_with('!') {
        token.pop();
    }

    if token.is_empty() {
        return None;
    }

    if token.starts_with("http://") || token.starts_with("https://") {
        return Some(Hit::Url(token));
    }

    if token.starts_with("localhost:") || token.starts_with("127.0.0.1:") || token == "localhost" {
        let url = if token.starts_with("http://") || token.starts_with("https://") {
            token
        } else {
            format!("http://{}", token)
        };
        return Some(Hit::Url(url));
    }

    if is_probable_filepath(&token) {
        let (path, line, col) = parse_file_location(&token);
        return Some(Hit::FilePath { path, line, col });
    }

    if let Some(domain_url) = parse_bare_domain(&token) {
        return Some(Hit::Url(domain_url));
    }

    None
}

fn is_probable_filepath(token: &str) -> bool {

    let has_separator = token.contains('/') || token.contains('\\') || token.starts_with('.');
    let has_colon_line = token.contains(':') && token.chars().any(|c| c.is_ascii_digit());
    let has_file_ext = token.contains('.') && (
        token.ends_with(".rs") || token.ends_with(".ts") || token.ends_with(".tsx") ||
        token.ends_with(".js") || token.ends_with(".jsx") || token.ends_with(".json") ||
        token.ends_with(".toml") || token.ends_with(".md") || token.ends_with(".py") ||
        token.ends_with(".go") || token.ends_with(".c") || token.ends_with(".cpp") ||
        token.ends_with(".h") || token.ends_with(".sh") || token.ends_with(".yaml") ||
        token.ends_with(".yml") || token.ends_with(".css") || token.ends_with(".html")
    );

    has_separator || (has_file_ext && has_colon_line) || (has_file_ext && !is_known_tld_domain(token))
}

fn is_known_tld_domain(token: &str) -> bool {
    if let Some((domain, _)) = token.split_once('/') {
        if let Some((_, tld)) = domain.rsplit_once('.') {
            return KNOWN_TLDS.contains(&tld.to_ascii_lowercase().as_str());
        }
    }
    false
}

fn parse_file_location(token: &str) -> (String, Option<u32>, Option<u32>) {
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() >= 3 {
        if let (Ok(line), Ok(col)) = (parts[parts.len() - 2].parse::<u32>(), parts[parts.len() - 1].parse::<u32>()) {
            let path = parts[..parts.len() - 2].join(":");
            return (path, Some(line), Some(col));
        }
    }
    if parts.len() >= 2 {
        if let Ok(line) = parts[parts.len() - 1].parse::<u32>() {
            let path = parts[..parts.len() - 1].join(":");
            return (path, Some(line), None);
        }
    }
    (token.to_string(), None, None)
}

fn parse_bare_domain(token: &str) -> Option<String> {
    let (authority, _) = token.split_once('/').unwrap_or((token, ""));
    let (host, has_port) = match authority.split_once(':') {
        Some((h, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => {
            (h, true)
        }
        Some(_) => return None,
        None => (authority, false),
    };
    if (host == "localhost" || host == "127.0.0.1") && has_port {
        return Some(format!("http://{}", token));
    }
    if let Some((_, tld)) = host.rsplit_once('.') {
        let tld_lower = tld.to_ascii_lowercase();
        if KNOWN_TLDS.contains(&tld_lower.as_str()) && !tld_lower.is_empty() {
            return Some(format!("https://{}", token));
        }
    }
    None
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

pub fn open_file_in_editor(path: &str, _line: Option<u32>, cwd: Option<&str>) {
    let full_path = if std::path::Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else if let Some(cwd) = cwd {
        std::path::Path::new(cwd).join(path)
    } else {
        std::path::PathBuf::from(path)
    };

    #[cfg(target_os = "macos")]
    {
        let editor = std::env::var("EDITOR").unwrap_or_default();
        if !editor.is_empty() {
            let _ = std::process::Command::new(&editor).arg(&full_path).spawn();
        } else {
            let _ = std::process::Command::new("open").arg(&full_path).spawn();
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(&full_path).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_https_urls() {
        let line = "Visit https://github.com/commandcode/plexus for details.";
        assert_eq!(
            link_at(line, 10),
            Some(Hit::Url("https://github.com/commandcode/plexus".into()))
        );
        assert_eq!(
            link_at(line, 35),
            Some(Hit::Url("https://github.com/commandcode/plexus".into()))
        );
    }

    #[test]
    fn detects_localhost_with_port() {
        let line = "Server running on http://localhost:3000 ready";
        assert_eq!(
            link_at(line, 25),
            Some(Hit::Url("http://localhost:3000".into()))
        );

        let bare = "Ready at localhost:5173!";
        assert_eq!(
            link_at(bare, 12),
            Some(Hit::Url("http://localhost:5173".into()))
        );
    }

    #[test]
    fn detects_file_paths_with_line_and_col() {
        let line = "error in src/pane.rs:42:10: cannot borrow";
        assert_eq!(
            link_at(line, 14),
            Some(Hit::FilePath {
                path: "src/pane.rs".into(),
                line: Some(42),
                col: Some(10),
            })
        );
    }

    #[test]
    fn detects_file_paths_with_line_only() {
        let line = "--> tests/input_tests.rs:25";
        assert_eq!(
            link_at(line, 8),
            Some(Hit::FilePath {
                path: "tests/input_tests.rs".into(),
                line: Some(25),
                col: None,
            })
        );
    }

    #[test]
    fn ignores_ordinary_words_and_spaces() {
        let line = "Just a normal sentence with no links.";
        assert_eq!(link_at(line, 0), None);
        assert_eq!(link_at(line, 10), None);
        assert_eq!(link_at(line, 6), None);
    }

    #[test]
    fn detects_localhost_dev_servers() {
        let line = "Server running on localhost:3000/api";
        assert_eq!(
            link_at(line, 20),
            Some(Hit::Url("http://localhost:3000/api".into()))
        );
    }
}

