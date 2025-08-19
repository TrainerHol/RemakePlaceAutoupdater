use anyhow::Error;
use serde::Serialize;

/// Categories of errors that can occur in the application
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ErrorCategory {
    Network,
    FileSystem,
    Permission,
    Validation,
    Configuration,
    Archive,
    Unknown,
}

/// Enhanced error information with user-friendly details
#[derive(Debug, Clone, Serialize)]
pub struct ErrorInfo {
    pub category: ErrorCategory,
    pub user_message: String,
    pub technical_details: String,
    pub recovery_suggestion: String,
    pub is_retryable: bool,
}

/// Handles error categorization and provides user-friendly messages
pub struct ErrorHandler;

impl ErrorHandler {
    /// Categorizes an error and provides detailed information for the user
    pub fn categorize_error(error: &Error) -> ErrorInfo {
        let error_str = error.to_string().to_lowercase();
        let error_chain = error.chain().map(|e| e.to_string()).collect::<Vec<_>>().join(" | ");
        
        // Network-related errors
        if Self::is_network_error(&error_str) {
            return Self::create_network_error_info(&error_str, &error_chain);
        }
        
        // File system errors
        if Self::is_filesystem_error(&error_str) {
            return Self::create_filesystem_error_info(&error_str, &error_chain);
        }
        
        // Permission errors
        if Self::is_permission_error(&error_str) {
            return Self::create_permission_error_info(&error_str, &error_chain);
        }
        
        // Validation errors
        if Self::is_validation_error(&error_str) {
            return Self::create_validation_error_info(&error_str, &error_chain);
        }
        
        // Configuration errors
        if Self::is_configuration_error(&error_str) {
            return Self::create_configuration_error_info(&error_str, &error_chain);
        }
        
        // Archive extraction errors
        if Self::is_archive_error(&error_str) {
            return Self::create_archive_error_info(&error_str, &error_chain);
        }
        
        // Unknown/generic errors
        Self::create_unknown_error_info(&error_chain)
    }
    
    fn is_network_error(error_str: &str) -> bool {
        error_str.contains("network") ||
        error_str.contains("connection") ||
        error_str.contains("timeout") ||
        error_str.contains("timed out") ||
        error_str.contains("dns") ||
        error_str.contains("host") ||
        error_str.contains("unreachable") ||
        error_str.contains("refused") ||
        error_str.contains("reset") ||
        error_str.contains("broken pipe") ||
        error_str.contains("502") ||
        error_str.contains("503") ||
        error_str.contains("504") ||
        error_str.contains("gateway timeout") ||
        error_str.contains("service unavailable") ||
        error_str.contains("bad gateway")
    }
    
    fn is_filesystem_error(error_str: &str) -> bool {
        error_str.contains("no such file") ||
        error_str.contains("file not found") ||
        error_str.contains("directory not found") ||
        error_str.contains("disk") ||
        error_str.contains("space") ||
        error_str.contains("full") ||
        error_str.contains("read-only") ||
        error_str.contains("invalid path") ||
        error_str.contains("path too long") ||
        error_str.contains("io error")
    }
    
    fn is_permission_error(error_str: &str) -> bool {
        error_str.contains("permission denied") ||
        error_str.contains("access denied") ||
        error_str.contains("unauthorized") ||
        error_str.contains("forbidden") ||
        error_str.contains("cannot write") ||
        error_str.contains("cannot read") ||
        error_str.contains("cannot create")
    }
    
    fn is_validation_error(error_str: &str) -> bool {
        error_str.contains("validation") ||
        error_str.contains("invalid") ||
        error_str.contains("corrupt") ||
        error_str.contains("checksum") ||
        error_str.contains("integrity") ||
        error_str.contains("malformed")
    }
    
    fn is_configuration_error(error_str: &str) -> bool {
        error_str.contains("config") ||
        error_str.contains("configuration") ||
        error_str.contains("setting") ||
        error_str.contains("missing") ||
        error_str.contains("not configured")
    }
    
    fn is_archive_error(error_str: &str) -> bool {
        error_str.contains("extract") ||
        error_str.contains("archive") ||
        error_str.contains("zip") ||
        error_str.contains("7z") ||
        error_str.contains("tar") ||
        error_str.contains("compression") ||
        error_str.contains("decompression") ||
        error_str.contains("zstd") ||
        error_str.contains("zst")
    }
    
    fn create_network_error_info(error_str: &str, technical_details: &str) -> ErrorInfo {
        let (user_message, recovery_suggestion, is_retryable) = if error_str.contains("timeout") || error_str.contains("timed out") {
            (
                "The connection timed out while downloading the update.",
                "Check your internet connection and try again. If the problem persists, try clearing the cache.",
                true
            )
        } else if error_str.contains("refused") || error_str.contains("unreachable") {
            (
                "Could not connect to the download server.",
                "Check your internet connection and firewall settings. The server may be temporarily unavailable.",
                true
            )
        } else if error_str.contains("dns") {
            (
                "Could not resolve the download server address.",
                "Check your internet connection and DNS settings. Try again in a few minutes.",
                true
            )
        } else if error_str.contains("502") || error_str.contains("503") || error_str.contains("504") {
            (
                "The download server is temporarily unavailable.",
                "This is usually temporary. Try again in a few minutes.",
                true
            )
        } else {
            (
                "A network error occurred while downloading the update.",
                "Check your internet connection and try again. If the problem persists, try clearing the cache.",
                true
            )
        };
        
        ErrorInfo {
            category: ErrorCategory::Network,
            user_message: user_message.to_string(),
            technical_details: technical_details.to_string(),
            recovery_suggestion: recovery_suggestion.to_string(),
            is_retryable,
        }
    }
    
    fn create_filesystem_error_info(error_str: &str, technical_details: &str) -> ErrorInfo {
        let (user_message, recovery_suggestion, is_retryable) = if error_str.contains("space") || error_str.contains("full") {
            (
                "Not enough disk space to complete the operation.",
                "Free up some disk space and try again.",
                false
            )
        } else if error_str.contains("file not found") || error_str.contains("directory not found") {
            (
                "A required file or directory could not be found.",
                "Check your installation path settings and try again.",
                false
            )
        } else if error_str.contains("read-only") {
            (
                "Cannot write to the selected location because it's read-only.",
                "Choose a different installation directory or change the folder permissions.",
                false
            )
        } else {
            (
                "A file system error occurred.",
                "Check your installation path and disk space, then try again.",
                false
            )
        };
        
        ErrorInfo {
            category: ErrorCategory::FileSystem,
            user_message: user_message.to_string(),
            technical_details: technical_details.to_string(),
            recovery_suggestion: recovery_suggestion.to_string(),
            is_retryable,
        }
    }
    
    fn create_permission_error_info(error_str: &str, technical_details: &str) -> ErrorInfo {
        let (user_message, recovery_suggestion) = if error_str.contains("permission denied") || error_str.contains("access denied") {
            (
                "Permission denied when accessing the installation directory.",
                "Run the launcher as administrator or choose a different installation directory."
            )
        } else if error_str.contains("unauthorized") || error_str.contains("forbidden") {
            (
                "Access to the installation directory is forbidden.",
                "Check folder permissions or run the launcher as administrator."
            )
        } else {
            (
                "Insufficient permissions to complete the operation.",
                "Run the launcher as administrator or check folder permissions."
            )
        };
        
        ErrorInfo {
            category: ErrorCategory::Permission,
            user_message: user_message.to_string(),
            technical_details: technical_details.to_string(),
            recovery_suggestion: recovery_suggestion.to_string(),
            is_retryable: false,
        }
    }
    
    fn create_validation_error_info(error_str: &str, technical_details: &str) -> ErrorInfo {
        let (user_message, recovery_suggestion) = if error_str.contains("corrupt") || error_str.contains("integrity") {
            (
                "The downloaded file appears to be corrupted.",
                "Clear the cache and try downloading again. If the problem persists, the server file may be corrupted."
            )
        } else if error_str.contains("checksum") {
            (
                "The downloaded file failed integrity verification.",
                "Clear the cache and try downloading again."
            )
        } else {
            (
                "The file or data failed validation.",
                "Clear the cache and try again. If the problem persists, contact support."
            )
        };
        
        ErrorInfo {
            category: ErrorCategory::Validation,
            user_message: user_message.to_string(),
            technical_details: technical_details.to_string(),
            recovery_suggestion: recovery_suggestion.to_string(),
            is_retryable: true,
        }
    }
    
    fn create_configuration_error_info(error_str: &str, technical_details: &str) -> ErrorInfo {
        let user_message = if error_str.contains("missing") || error_str.contains("not configured") {
            "Required configuration is missing or incomplete."
        } else {
            "There's an issue with the application configuration."
        };
        
        ErrorInfo {
            category: ErrorCategory::Configuration,
            user_message: user_message.to_string(),
            technical_details: technical_details.to_string(),
            recovery_suggestion: "Check your settings and reconfigure if necessary.".to_string(),
            is_retryable: false,
        }
    }
    
    fn create_archive_error_info(error_str: &str, technical_details: &str) -> ErrorInfo {
        let (user_message, recovery_suggestion) = if error_str.contains("zstd") || error_str.contains("zst") {
            (
                "Failed to extract the archive. The compression format may not be supported.",
                "Clear the cache and try downloading again. If the problem persists, the archive format may be unsupported."
            )
        } else if error_str.contains("extract") {
            (
                "Failed to extract the downloaded archive.",
                "The file may be corrupted. Clear the cache and try downloading again."
            )
        } else {
            (
                "An error occurred while processing the archive.",
                "Clear the cache and try downloading again."
            )
        };
        
        ErrorInfo {
            category: ErrorCategory::Archive,
            user_message: user_message.to_string(),
            technical_details: technical_details.to_string(),
            recovery_suggestion: recovery_suggestion.to_string(),
            is_retryable: true,
        }
    }
    
    fn create_unknown_error_info(technical_details: &str) -> ErrorInfo {
        ErrorInfo {
            category: ErrorCategory::Unknown,
            user_message: "An unexpected error occurred.".to_string(),
            technical_details: technical_details.to_string(),
            recovery_suggestion: "Try again. If the problem persists, contact support.".to_string(),
            is_retryable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_network_error_categorization() {
        let error = anyhow!("Connection timeout occurred");
        let info = ErrorHandler::categorize_error(&error);
        
        assert_eq!(info.category, ErrorCategory::Network);
        assert!(info.is_retryable);
        assert!(info.user_message.contains("timed out"));
    }

    #[test]
    fn test_permission_error_categorization() {
        let error = anyhow!("Permission denied when accessing file");
        let info = ErrorHandler::categorize_error(&error);
        
        assert_eq!(info.category, ErrorCategory::Permission);
        assert!(!info.is_retryable);
        assert!(info.user_message.contains("Permission denied"));
    }

    #[test]
    fn test_filesystem_error_categorization() {
        let error = anyhow!("No space left on device");
        let info = ErrorHandler::categorize_error(&error);
        
        assert_eq!(info.category, ErrorCategory::FileSystem);
        assert!(!info.is_retryable);
        assert!(info.user_message.contains("disk space"));
    }

    #[test]
    fn test_archive_error_categorization() {
        let error = anyhow!("Failed to extract zstd archive");
        let info = ErrorHandler::categorize_error(&error);
        
        assert_eq!(info.category, ErrorCategory::Archive);
        assert!(info.is_retryable);
        assert!(info.user_message.contains("extract"));
    }

    #[test]
    fn test_unknown_error_categorization() {
        let error = anyhow!("Some random error message");
        let info = ErrorHandler::categorize_error(&error);
        
        assert_eq!(info.category, ErrorCategory::Unknown);
        assert!(!info.is_retryable);
        assert!(info.user_message.contains("unexpected"));
    }
}