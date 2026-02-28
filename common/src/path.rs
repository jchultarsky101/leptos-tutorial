use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

use crate::error::CatalogError;

const MAX_PATH_LEN: usize = 4096;
const MAX_SEGMENT_LEN: usize = 255;

/// A validated, URI-compatible virtual path.
///
/// Paths always start with `/`. The root folder is exactly `/`.
/// Segments are separated by `/` and may not be empty, `.`, or `..`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = String, example = "/documents/reports")]
pub struct CatalogPath(String);

impl CatalogPath {
    /// Construct and validate a `CatalogPath` from a raw string.
    pub fn new(raw: &str) -> Result<Self, CatalogError> {
        if raw.len() > MAX_PATH_LEN {
            return Err(CatalogError::InvalidPath(format!(
                "path exceeds {MAX_PATH_LEN} characters"
            )));
        }
        if !raw.starts_with('/') {
            return Err(CatalogError::InvalidPath("path must start with '/'".into()));
        }
        if raw == "/" {
            return Ok(CatalogPath("/".to_owned()));
        }
        for segment in raw[1..].split('/') {
            validate_segment(segment, raw)?;
        }
        Ok(CatalogPath(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0 == "/"
    }

    /// Returns the parent path, or `None` if this is the root.
    pub fn parent(&self) -> Option<CatalogPath> {
        if self.is_root() {
            return None;
        }
        match self.0.rfind('/') {
            Some(0) => Some(CatalogPath("/".to_owned())),
            Some(i) => Some(CatalogPath(self.0[..i].to_owned())),
            None => None,
        }
    }

    /// Returns the last path segment (the "file name" or "folder name").
    /// Returns an empty string for the root path.
    pub fn name(&self) -> &str {
        if self.is_root() {
            return "";
        }
        self.0.rsplit('/').next().unwrap_or("")
    }

    /// Appends a single validated segment and returns a new path.
    pub fn join(&self, segment: &str) -> Result<CatalogPath, CatalogError> {
        if segment.contains('/') {
            return Err(CatalogError::InvalidPath(
                "segment must not contain '/'".into(),
            ));
        }
        validate_segment(segment, segment)?;
        if self.is_root() {
            Ok(CatalogPath(format!("/{segment}")))
        } else {
            Ok(CatalogPath(format!("{}/{segment}", self.0)))
        }
    }

    /// Returns `true` if `self` is equal to `prefix` or is a direct/indirect
    /// child of `prefix`. Used for recursive delete checks.
    pub fn starts_with_folder(&self, prefix: &CatalogPath) -> bool {
        if prefix.is_root() {
            return true;
        }
        if self == prefix {
            return true;
        }
        self.0.starts_with(&format!("{}/", prefix.0))
    }
}

fn validate_segment(segment: &str, context: &str) -> Result<(), CatalogError> {
    if segment.is_empty() {
        return Err(CatalogError::InvalidPath(format!(
            "path contains an empty segment: {context:?}"
        )));
    }
    if segment == ".." || segment == "." {
        return Err(CatalogError::InvalidPath(format!(
            "path contains illegal segment {segment:?} in {context:?}"
        )));
    }
    if segment.len() > MAX_SEGMENT_LEN {
        return Err(CatalogError::InvalidPath(format!(
            "segment exceeds {MAX_SEGMENT_LEN} characters in {context:?}"
        )));
    }
    if segment.contains('\0') || segment.chars().any(|c| c.is_control()) {
        return Err(CatalogError::InvalidPath(format!(
            "segment contains invalid characters in {context:?}"
        )));
    }
    Ok(())
}

impl<'de> Deserialize<'de> for CatalogPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        CatalogPath::new(&s).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for CatalogPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_valid() {
        assert!(CatalogPath::new("/").is_ok());
    }

    #[test]
    fn simple_path_is_valid() {
        let p = CatalogPath::new("/docs/reports").unwrap();
        assert_eq!(p.as_str(), "/docs/reports");
    }

    #[test]
    fn must_start_with_slash() {
        assert!(matches!(
            CatalogPath::new("docs"),
            Err(CatalogError::InvalidPath(_))
        ));
    }

    #[test]
    fn empty_segment_rejected() {
        assert!(matches!(
            CatalogPath::new("/docs//reports"),
            Err(CatalogError::InvalidPath(_))
        ));
    }

    #[test]
    fn dotdot_rejected() {
        assert!(matches!(
            CatalogPath::new("/docs/../etc"),
            Err(CatalogError::InvalidPath(_))
        ));
    }

    #[test]
    fn dot_rejected() {
        assert!(matches!(
            CatalogPath::new("/docs/./etc"),
            Err(CatalogError::InvalidPath(_))
        ));
    }

    #[test]
    fn trailing_slash_rejected() {
        assert!(matches!(
            CatalogPath::new("/docs/"),
            Err(CatalogError::InvalidPath(_))
        ));
    }

    #[test]
    fn control_char_rejected() {
        assert!(matches!(
            CatalogPath::new("/docs/rep\x00ort"),
            Err(CatalogError::InvalidPath(_))
        ));
    }

    #[test]
    fn parent_of_root_is_none() {
        assert!(CatalogPath::new("/").unwrap().parent().is_none());
    }

    #[test]
    fn parent_of_first_level() {
        let p = CatalogPath::new("/docs").unwrap();
        assert_eq!(p.parent().unwrap().as_str(), "/");
    }

    #[test]
    fn parent_of_nested() {
        let p = CatalogPath::new("/docs/reports").unwrap();
        assert_eq!(p.parent().unwrap().as_str(), "/docs");
    }

    #[test]
    fn name_of_root_is_empty() {
        assert_eq!(CatalogPath::new("/").unwrap().name(), "");
    }

    #[test]
    fn name_of_nested() {
        assert_eq!(CatalogPath::new("/docs/reports").unwrap().name(), "reports");
    }

    #[test]
    fn join_appends_segment() {
        let p = CatalogPath::new("/docs").unwrap();
        assert_eq!(p.join("reports").unwrap().as_str(), "/docs/reports");
    }

    #[test]
    fn join_root_appends_segment() {
        let root = CatalogPath::new("/").unwrap();
        assert_eq!(root.join("docs").unwrap().as_str(), "/docs");
    }

    #[test]
    fn join_rejects_slash_in_segment() {
        let p = CatalogPath::new("/docs").unwrap();
        assert!(p.join("a/b").is_err());
    }

    #[test]
    fn starts_with_folder_self() {
        let p = CatalogPath::new("/docs").unwrap();
        assert!(p.starts_with_folder(&p));
    }

    #[test]
    fn starts_with_folder_child() {
        let parent = CatalogPath::new("/docs").unwrap();
        let child = CatalogPath::new("/docs/reports").unwrap();
        assert!(child.starts_with_folder(&parent));
    }

    #[test]
    fn starts_with_folder_no_prefix_match() {
        let a = CatalogPath::new("/documents").unwrap();
        let b = CatalogPath::new("/docs").unwrap();
        assert!(!a.starts_with_folder(&b));
    }

    #[test]
    fn starts_with_root_always_true() {
        let root = CatalogPath::new("/").unwrap();
        let p = CatalogPath::new("/anything/deep").unwrap();
        assert!(p.starts_with_folder(&root));
    }

    #[test]
    fn path_too_long_rejected() {
        let long = format!("/{}", "a".repeat(MAX_PATH_LEN));
        assert!(matches!(
            CatalogPath::new(&long),
            Err(CatalogError::InvalidPath(_))
        ));
    }
}
