use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::types::Ticket;

/// Discovers relevant files based on the ticket description.
///
/// 1. Tokenizes the description.
/// 2. Walks the workspace (respecting exclusions).
/// 3. Scores files:
///    - High priority: filename matches a token.
///    - Medium priority: file content contains a token.
pub fn discover_context(ticket: &Ticket, root: &Path) -> Vec<String> {
    let tokens = tokenize(&ticket.spec.description);
    if tokens.is_empty() {
        return vec![];
    }

    let mut scored_files: Vec<(String, u32)> = Vec::new();
    let ignore_patterns = vec![
        "target/",
        "node_modules/",
        ".git/",
        "dist/",
        "build/",
        ".lock",
        "package-lock.json",
        "yarn.lock",
    ];

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip ignored paths
        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            ignore_patterns.iter().any(|pat| s.contains(&pat.replace("/", "")))
        }) {
            continue;
        }

        let rel_path = match path.strip_prefix(root) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        // Ensure forward slashes for consistency
        let rel_path_normalized = rel_path.replace("\\", "/");

        let mut score = 0;
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        // 1. Filename Match (High Priority)
        for token in &tokens {
            if file_name.contains(token) {
                score += 10;
            }
        }

        // 2. Content Match (Medium Priority)
        if score < 10 { // Optimization: if already high priority, maybe skip content check?
                        // Actually, description says "Content Match: Medium Priority".
                        // Let's do it if it's not binary binary large object.

            // Basic check to avoid reading huge binary files
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if ["rs", "ts", "tsx", "js", "toml", "json", "md", "css", "html"].contains(&ext_str.as_ref()) {
                     if let Ok(content) = fs::read_to_string(path) {
                        for token in &tokens {
                            if content.contains(token) {
                                score += 1;
                                // Break early if we just want "any match" or count all matches?
                                // "If a file contains the token inside its content, it gets Medium Priority"
                                // Let's just add 1 per token found.
                            }
                        }
                     }
                }
            }
        }

        if score > 0 {
            scored_files.push((rel_path_normalized, score));
        }
    }

    // Sort by score descending
    scored_files.sort_by(|a, b| b.1.cmp(&a.1));

    // Return just the paths
    scored_files.into_iter().map(|(path, _)| path).collect()
}

fn tokenize(text: &str) -> HashSet<String> {
    let stop_words: HashSet<&str> = [
        "the", "and", "a", "an", "to", "in", "of", "for", "with", "on", "at",
        "by", "from", "up", "about", "into", "over", "after", "implement", "update",
        "create", "add", "fix", "remove", "delete", "refactor", "change", "modify",
        "use", "using", "ensure", "make", "is", "are", "was", "were", "be", "been",
        "can", "could", "should", "would", "will", "may", "might", "must", "have", "has", "had",
        "do", "does", "did", "todo", "done", "spec", "ticket", "description", "title", "status", "priority"
    ].iter().cloned().collect();

    text.split_whitespace()
        .map(|s| {
            // Remove punctuation
            s.chars()
             .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
             .collect::<String>()
             .to_lowercase()
        })
        .filter(|s| !s.is_empty() && !stop_words.contains(s.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let text = "Update the KanbanBoard component to use a new API.";
        let tokens = tokenize(text);

        assert!(tokens.contains("kanbanboard"));
        assert!(tokens.contains("component"));
        assert!(tokens.contains("new"));
        assert!(tokens.contains("api"));
        assert!(!tokens.contains("update")); // stop word
        assert!(!tokens.contains("the")); // stop word
        assert!(!tokens.contains("to")); // stop word
    }
}
