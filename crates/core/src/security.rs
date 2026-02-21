//! Security hardening: ZIP bomb protection, path traversal guards, resource limits.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::SecurityError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityLimits {
    /// Maximum decompression ratio before flagging as ZIP bomb.
    pub max_compression_ratio: u64,
    /// Maximum number of files allowed in an archive.
    pub max_file_count: u64,
    /// Maximum size of a single resource in bytes.
    pub max_resource_size_bytes: u64,
    /// Maximum total decompressed size in bytes.
    pub max_total_size_bytes: u64,
    /// Maximum XML/HTML nesting depth.
    pub max_nesting_depth: u32,
    /// Maximum parse time in seconds.
    pub max_parse_seconds: u64,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_compression_ratio: 100,
            max_file_count: 10_000,
            max_resource_size_bytes: 200 * 1024 * 1024, // 200 MB
            max_total_size_bytes: 1024 * 1024 * 1024,   // 1 GB
            max_nesting_depth: 200,
            max_parse_seconds: 300,
        }
    }
}

/// Check a ZIP archive entry path for path traversal attacks.
/// Returns an error if the path contains `..` components or absolute paths.
pub fn check_path_traversal(entry_path: &str) -> Result<(), SecurityError> {
    // Check for absolute paths
    if entry_path.starts_with('/') || entry_path.starts_with('\\') {
        return Err(SecurityError::PathTraversal {
            path: entry_path.to_string(),
        });
    }

    // Check for Windows absolute paths (e.g., C:\)
    if entry_path.len() >= 2 && entry_path.as_bytes()[1] == b':' {
        return Err(SecurityError::PathTraversal {
            path: entry_path.to_string(),
        });
    }

    // Check each component for parent directory references
    for component in Path::new(entry_path).components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(SecurityError::PathTraversal {
                    path: entry_path.to_string(),
                });
            }
            _ => {}
        }
    }

    Ok(())
}

/// Check if a decompression ratio exceeds the configured limit (ZIP bomb detection).
pub fn check_compression_ratio(
    compressed_size: u64,
    uncompressed_size: u64,
    limits: &SecurityLimits,
) -> Result<(), SecurityError> {
    if compressed_size == 0 {
        if uncompressed_size > 0 {
            return Err(SecurityError::ZipBomb {
                ratio: u64::MAX,
                limit: limits.max_compression_ratio,
            });
        }
        return Ok(());
    }

    let ratio = uncompressed_size / compressed_size;
    if ratio > limits.max_compression_ratio {
        return Err(SecurityError::ZipBomb {
            ratio,
            limit: limits.max_compression_ratio,
        });
    }

    Ok(())
}

/// Check if the number of files in an archive exceeds the limit.
pub fn check_file_count(count: u64, limits: &SecurityLimits) -> Result<(), SecurityError> {
    if count > limits.max_file_count {
        return Err(SecurityError::TooManyFiles {
            count,
            limit: limits.max_file_count,
        });
    }
    Ok(())
}

/// Check if a single resource exceeds the size limit.
pub fn check_resource_size(
    name: &str,
    size_bytes: u64,
    limits: &SecurityLimits,
) -> Result<(), SecurityError> {
    if size_bytes > limits.max_resource_size_bytes {
        return Err(SecurityError::OversizedResource {
            name: name.to_string(),
            size_mb: size_bytes / (1024 * 1024),
            limit_mb: limits.max_resource_size_bytes / (1024 * 1024),
        });
    }
    Ok(())
}

/// Check if total decompressed size exceeds the limit.
pub fn check_total_size(
    total_bytes: u64,
    limits: &SecurityLimits,
) -> Result<(), SecurityError> {
    if total_bytes > limits.max_total_size_bytes {
        return Err(SecurityError::OversizedResource {
            name: "<total>".to_string(),
            size_mb: total_bytes / (1024 * 1024),
            limit_mb: limits.max_total_size_bytes / (1024 * 1024),
        });
    }
    Ok(())
}

/// Check if XML/HTML nesting depth exceeds the limit.
pub fn check_nesting_depth(depth: u32, limits: &SecurityLimits) -> Result<(), SecurityError> {
    if depth > limits.max_nesting_depth {
        return Err(SecurityError::ExcessiveNesting {
            depth,
            limit: limits.max_nesting_depth,
        });
    }
    Ok(())
}

/// Check an EPUB's encryption.xml for DRM.
/// Returns an error with the DRM type if DRM is detected.
pub fn check_epub_drm(encryption_xml: &str) -> Result<(), SecurityError> {
    // Adobe DRM namespace
    if encryption_xml.contains("http://ns.adobe.com/adept")
        || encryption_xml.contains("http://ns.adobe.com/digitaleditions")
    {
        return Err(SecurityError::DrmProtected {
            format: "EPUB".to_string(),
            drm_type: "Adobe DRM".to_string(),
        });
    }

    // Apple FairPlay DRM
    if encryption_xml.contains("http://www.apple.com/ibooks")
        || encryption_xml.contains("sinf")
    {
        return Err(SecurityError::DrmProtected {
            format: "EPUB".to_string(),
            drm_type: "Apple FairPlay".to_string(),
        });
    }

    // Sony URMS
    if encryption_xml.contains("http://urms.org") {
        return Err(SecurityError::DrmProtected {
            format: "EPUB".to_string(),
            drm_type: "Sony URMS".to_string(),
        });
    }

    // Generic EncryptedData without known font obfuscation algorithms
    // (Font obfuscation is NOT DRM — it's allowed)
    if encryption_xml.contains("EncryptedData") {
        let is_font_obfuscation = encryption_xml
            .contains("http://www.idpf.org/2008/embedding")
            || encryption_xml.contains("http://ns.adobe.com/pdf/enc#RC");

        if !is_font_obfuscation {
            // Has encryption but it's not font obfuscation — likely DRM
            return Err(SecurityError::DrmProtected {
                format: "EPUB".to_string(),
                drm_type: "Unknown DRM".to_string(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_safe_paths() {
        assert!(check_path_traversal("content/chapter1.xhtml").is_ok());
        assert!(check_path_traversal("META-INF/container.xml").is_ok());
        assert!(check_path_traversal("mimetype").is_ok());
        assert!(check_path_traversal("OEBPS/images/cover.jpg").is_ok());
    }

    #[test]
    fn test_path_traversal_attacks() {
        assert!(check_path_traversal("../../../etc/passwd").is_err());
        assert!(check_path_traversal("content/../../etc/shadow").is_err());
        assert!(check_path_traversal("/etc/passwd").is_err());
        assert!(check_path_traversal("\\Windows\\System32\\config").is_err());
    }

    #[test]
    fn test_path_traversal_windows_absolute() {
        assert!(check_path_traversal("C:\\Windows\\System32").is_err());
    }

    #[test]
    fn test_compression_ratio_ok() {
        let limits = SecurityLimits::default();
        assert!(check_compression_ratio(1000, 50_000, &limits).is_ok()); // 50:1
    }

    #[test]
    fn test_compression_ratio_bomb() {
        let limits = SecurityLimits::default();
        assert!(check_compression_ratio(100, 100_000, &limits).is_err()); // 1000:1
    }

    #[test]
    fn test_compression_ratio_zero_compressed() {
        let limits = SecurityLimits::default();
        assert!(check_compression_ratio(0, 0, &limits).is_ok());
        assert!(check_compression_ratio(0, 100, &limits).is_err());
    }

    #[test]
    fn test_file_count_ok() {
        let limits = SecurityLimits::default();
        assert!(check_file_count(100, &limits).is_ok());
        assert!(check_file_count(10_000, &limits).is_ok());
    }

    #[test]
    fn test_file_count_exceeded() {
        let limits = SecurityLimits::default();
        assert!(check_file_count(10_001, &limits).is_err());
    }

    #[test]
    fn test_resource_size_ok() {
        let limits = SecurityLimits::default();
        assert!(check_resource_size("image.jpg", 1024 * 1024, &limits).is_ok());
    }

    #[test]
    fn test_resource_size_exceeded() {
        let limits = SecurityLimits::default();
        let size = 201 * 1024 * 1024; // 201 MB
        assert!(check_resource_size("huge.png", size, &limits).is_err());
    }

    #[test]
    fn test_nesting_depth_ok() {
        let limits = SecurityLimits::default();
        assert!(check_nesting_depth(50, &limits).is_ok());
        assert!(check_nesting_depth(200, &limits).is_ok());
    }

    #[test]
    fn test_nesting_depth_exceeded() {
        let limits = SecurityLimits::default();
        assert!(check_nesting_depth(201, &limits).is_err());
    }

    #[test]
    fn test_epub_drm_no_drm() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
</encryption>"#;
        assert!(check_epub_drm(xml).is_ok());
    }

    #[test]
    fn test_epub_drm_adobe() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <EncryptedData xmlns="http://www.w3.org/2001/04/xmlenc#">
    <KeyInfo xmlns="http://www.w3.org/2000/09/xmldsig#">
      <resource xmlns="http://ns.adobe.com/adept"/>
    </KeyInfo>
  </EncryptedData>
</encryption>"#;
        let err = check_epub_drm(xml).unwrap_err();
        match err {
            SecurityError::DrmProtected { drm_type, .. } => {
                assert_eq!(drm_type, "Adobe DRM");
            }
            _ => panic!("Expected DrmProtected error"),
        }
    }

    #[test]
    fn test_epub_drm_font_obfuscation_allowed() {
        // Font obfuscation is NOT DRM
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <EncryptedData xmlns="http://www.w3.org/2001/04/xmlenc#">
    <EncryptionMethod Algorithm="http://www.idpf.org/2008/embedding"/>
  </EncryptedData>
</encryption>"#;
        assert!(check_epub_drm(xml).is_ok());
    }
}
