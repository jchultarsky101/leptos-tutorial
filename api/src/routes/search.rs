use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use common::dto::{SearchResultDto, SearchResultsDto};
use serde::Deserialize;
use tokio::io::AsyncReadExt;
use utoipa::IntoParams;

use crate::{error::ApiError, state::AppState};

/// Maximum file size (bytes) we will read for content search.
const MAX_CONTENT_BYTES: u64 = 1_048_576; // 1 MiB

/// Maximum number of results returned.
const MAX_RESULTS: usize = 50;

/// Character radius around a content match for the snippet.
const SNIPPET_RADIUS: usize = 50;

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchQuery {
    /// The search term.
    pub q: String,
    /// Enable fuzzy matching (default: false).
    pub fuzzy: Option<bool>,
}

/// Returns `true` if `haystack` contains `needle` case-insensitively.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    let h = haystack.to_lowercase();
    let n = needle.to_lowercase();
    h.contains(&n)
}

/// Returns `true` if all characters of `needle` appear in `haystack` in order
/// (case-insensitive). This is the classic fuzzy/subsequence match.
fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let mut h_iter = haystack.chars().flat_map(|c| c.to_lowercase());
    for nc in needle.chars().flat_map(|c| c.to_lowercase()) {
        loop {
            match h_iter.next() {
                Some(hc) if hc == nc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Returns `true` if the MIME type is text-based and eligible for content search.
fn is_text_content_type(ct: &str) -> bool {
    ct.starts_with("text/")
        || ct == "application/json"
        || ct == "application/xml"
        || ct == "application/javascript"
}

/// Find a content match and extract a snippet. Returns `None` if no match.
fn find_content_match(content: &str, query: &str, fuzzy: bool) -> Option<String> {
    if fuzzy {
        // For fuzzy content search, just check if the subsequence exists.
        // Snippet: use the first char match location as the anchor.
        let lower_content = content.to_lowercase();
        let mut chars = query.chars().flat_map(|c| c.to_lowercase()).peekable();
        let first_char = *chars.peek()?;
        let first_pos = lower_content.find(first_char)?;
        // Verify full fuzzy match exists
        if !fuzzy_match(content, query) {
            return None;
        }
        Some(extract_snippet(content, first_pos))
    } else {
        let lower = content.to_lowercase();
        let needle = query.to_lowercase();
        let pos = lower.find(&needle)?;
        Some(extract_snippet(content, pos))
    }
}

/// Extract a ±SNIPPET_RADIUS character window around `pos`.
fn extract_snippet(content: &str, pos: usize) -> String {
    let start = pos.saturating_sub(SNIPPET_RADIUS);
    let end = (pos + SNIPPET_RADIUS).min(content.len());

    // Align to char boundaries
    let start = content
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= start)
        .unwrap_or(0);
    let end = content
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= end)
        .unwrap_or(content.len());

    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(&content[start..end]);
    if end < content.len() {
        snippet.push_str("...");
    }
    snippet
}

/// Search files and folders by name, with optional full-text content search.
#[utoipa::path(
    get,
    path = "/search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results", body = SearchResultsDto),
        (status = 422, description = "Missing query", body = common::dto::ErrorResponse),
    ),
    tag = "search"
)]
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let query = params.q.trim().to_owned();
    if query.is_empty() {
        return Err(ApiError::from(common::CatalogError::Validation(
            "search query must not be empty".into(),
        )));
    }
    let fuzzy = params.fuzzy.unwrap_or(false);

    let (folders, files) = state.metadata.search_entries().await?;
    let mut results: Vec<SearchResultDto> = Vec::new();

    let name_matches = |name: &str| -> bool {
        if fuzzy {
            fuzzy_match(name, &query)
        } else {
            contains_ci(name, &query)
        }
    };

    // ── Folder name matches ──────────────────────────────────────────────────
    for folder in &folders {
        if results.len() >= MAX_RESULTS {
            break;
        }
        if folder.path.is_root() {
            continue;
        }
        let name = folder.path.name().to_owned();
        if name_matches(&name) {
            results.push(SearchResultDto {
                path: folder.path.clone(),
                kind: "folder".into(),
                name,
                size_bytes: None,
                content_type: None,
                match_source: "name".into(),
                snippet: None,
            });
        }
    }

    // ── File name + content matches ──────────────────────────────────────────
    for file in &files {
        if results.len() >= MAX_RESULTS {
            break;
        }
        let name = file.path.name().to_owned();
        let name_hit = name_matches(&name);

        // Content search for text-based files
        let mut content_snippet: Option<String> = None;
        if is_text_content_type(&file.content_type)
            && file.size_bytes <= MAX_CONTENT_BYTES
            && let Ok(fh) = state.files.read(&file.path).await
        {
            let mut buf = String::new();
            let mut reader = tokio::io::BufReader::new(fh);
            if reader.read_to_string(&mut buf).await.is_ok() {
                content_snippet = find_content_match(&buf, &query, fuzzy);
            }
        }

        let match_source = match (name_hit, content_snippet.is_some()) {
            (true, true) => "both",
            (true, false) => "name",
            (false, true) => "content",
            (false, false) => continue,
        };

        results.push(SearchResultDto {
            path: file.path.clone(),
            kind: "file".into(),
            name,
            size_bytes: Some(file.size_bytes),
            content_type: Some(file.content_type.clone()),
            match_source: match_source.into(),
            snippet: content_snippet,
        });
    }

    Ok(Json(SearchResultsDto {
        query,
        fuzzy,
        results,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_contains_case_insensitive() {
        assert!(contains_ci("Report.txt", "repo"));
        assert!(contains_ci("README.md", "readme"));
        assert!(!contains_ci("hello", "xyz"));
    }

    #[test]
    fn fuzzy_subsequence() {
        assert!(fuzzy_match("report.txt", "rpt"));
        assert!(fuzzy_match("README.md", "rdm"));
        assert!(!fuzzy_match("hello", "xyz"));
        assert!(fuzzy_match("Report", "RPT"));
    }

    #[test]
    fn text_content_types() {
        assert!(is_text_content_type("text/plain"));
        assert!(is_text_content_type("text/csv"));
        assert!(is_text_content_type("text/markdown"));
        assert!(is_text_content_type("application/json"));
        assert!(!is_text_content_type("application/octet-stream"));
        assert!(!is_text_content_type("image/png"));
    }

    #[test]
    fn snippet_extraction() {
        let content = "The quick brown fox jumps over the lazy dog";
        let snippet = extract_snippet(content, 10);
        assert!(snippet.contains("brown"));
    }

    #[test]
    fn content_match_exact() {
        let content = "This file contains important data about quarterly reports.";
        let result = find_content_match(content, "quarterly", false);
        assert!(result.is_some());
        assert!(result.as_ref().is_some_and(|s| s.contains("quarterly")));
    }

    #[test]
    fn content_match_fuzzy() {
        let content = "quarterly report summary";
        let result = find_content_match(content, "qrs", true);
        assert!(result.is_some());
    }

    #[test]
    fn content_no_match() {
        let content = "hello world";
        assert!(find_content_match(content, "xyz", false).is_none());
        assert!(find_content_match(content, "xyz", true).is_none());
    }
}
