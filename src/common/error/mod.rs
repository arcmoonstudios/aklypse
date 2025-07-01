/* src/common/error/mod.rs */
#![warn(missing_docs)]
//! **Brief:** Main AklypseError enum (Snafu-based) with extensions and exports.
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
//! + [Error Handling Framework]
//!  - [Core Error Types]
//!  - [Error Propagation]
//!  - [Context Management]
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
// **GitHub:** [ArcMoon Studios](https://github.com/arcmoonstudios)
// **Copyright:** (c) 2025 ArcMoon Studios
// **Author:** Lord Xyn
// **License:** MIT

pub mod circuitbreaker;
pub mod decrust;
pub mod reporter;
pub mod types;

use snafu::{self, prelude::*, Backtrace, ErrorCompat, Snafu};
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Duration;
use std::fmt;

// Re-export key types from submodules
pub use self::types::{
    ErrorContext, ErrorSource, ErrorSeverity, ErrorCategory, DiagnosticResult,
    Autocorrection, FixType, FixDetails,
};
pub use self::reporter::{ErrorReporter, ErrorReportConfig, ErrorReportFormat};
pub use self::circuitbreaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, CircuitBreakerObserver
};
pub use self::decrust::{Decrust, AutocorrectableError};

/// A Result type specialized for AklypseError
pub type Result<T, E = AklypseError> = std::result::Result<T, E>;

/// Unified error type for Aklypse, based on Snafu.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum AklypseError {
    /// I/O related errors
    Io {
        source: Arc<std::io::Error>,
        path: Option<PathBuf>,
        operation: String,
        backtrace: snafu::Backtrace,
    },
    
    /// Parsing errors (JSON, YAML, etc.)
    Parse {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        kind: String,
        context_info: String,
        backtrace: snafu::Backtrace,
    },
    
    /// Network related errors
    Network {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        url: Option<String>,
        kind: String,
        backtrace: snafu::Backtrace,
    },
    
    /// Configuration related errors
    Config {
        message: String,
        path: Option<PathBuf>,
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        backtrace: snafu::Backtrace,
    },
    
    /// Validation errors
    Validation {
        field: String,
        message: String,
        backtrace: snafu::Backtrace,
    },
    
    /// Internal errors
    Internal {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        backtrace: snafu::Backtrace,
    },
    
    /// Circuit breaker is open
    CircuitBreakerOpen {
        name: String,
        retry_after: Option<Duration>,
        backtrace: snafu::Backtrace,
    },
    
    /// Operation timed out
    Timeout {
        operation: String,
        duration: Duration,
        backtrace: snafu::Backtrace,
    },
    
    /// Resource exhaustion
    ResourceExhausted {
        resource: String,
        limit: String,
        current: String,
        backtrace: snafu::Backtrace,
    },
    
    /// Resource not found
    NotFound {
        resource_type: String,
        identifier: String,
        backtrace: snafu::Backtrace,
    },
    
    /// State conflict
    StateConflict {
        message: String,
        backtrace: snafu::Backtrace,
    },
    
    /// Concurrency related errors
    Concurrency {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        backtrace: snafu::Backtrace,
    },
    
    /// External service errors
    ExternalService {
        service_name: String,
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        backtrace: snafu::Backtrace,
    },
    
    /// Missing value errors
    MissingValue {
        item_description: String,
        backtrace: snafu::Backtrace,
    },
    
    /// Multiple errors
    MultipleErrors {
        errors: Vec<AklypseError>,
        backtrace: snafu::Backtrace,
    },
    
    /// Error with rich context
    WithRichContext {
        context: types::ErrorContext,
        source: Box<AklypseError>,
        backtrace: snafu::Backtrace,
    },
    
    /// General purpose error wrapper
    Whatever {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        backtrace: Option<snafu::Backtrace>,
    },
}

impl Clone for AklypseError {
    fn clone(&self) -> Self {
        match self {
            Self::Io { source, path, operation, .. } => {
                // Properly preserve the original error kind and message when cloning
                let source_clone = Arc::new(std::io::Error::new(
                    source.kind(),
                    format!("{}", source),
                ));
                IoSnafu {
                    source: source_clone,
                    path: path.clone(),
                    operation: operation.clone(),
                }.build()
            },
            Self::Parse { source, kind, context_info, .. } => {
                let source_message = format!("{}", source);
                ParseSnafu {
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        source_message,
                    )),
                    kind: kind.clone(),
                    context_info: context_info.clone(),
                }.build()
            },
            Self::Network { source, url, kind, .. } => {
                let source_message = format!("{}", source);
                NetworkSnafu {
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        source_message,
                    )),
                    url: url.clone(),
                    kind: kind.clone(),
                }.build()
            },
            Self::Config { message, path, source, .. } => {
                let cloned_source = source.as_ref().map(|s| {
                    let msg = format!("{}", s);
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)) as Box<dyn std::error::Error + Send + Sync>
                });
                ConfigSnafu {
                    message: message.clone(),
                    path: path.clone(),
                    source: cloned_source,
                }.build()
            },
            Self::Validation { field, message, .. } => {
                ValidationSnafu {
                    field: field.clone(),
                    message: message.clone(),
                }.build()
            },
            Self::Internal { message, source, .. } => {
                let cloned_source = source.as_ref().map(|s| {
                    let msg = format!("{}", s);
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)) as Box<dyn std::error::Error + Send + Sync>
                });
                InternalSnafu {
                    message: message.clone(),
                    source: cloned_source,
                }.build()
            },
            Self::CircuitBreakerOpen { name, retry_after, .. } => {
                CircuitBreakerOpenSnafu {
                    name: name.clone(),
                    retry_after: *retry_after,
                }.build()
            },
            Self::Timeout { operation, duration, .. } => {
                TimeoutSnafu {
                    operation: operation.clone(),
                    duration: *duration,
                }.build()
            },
            Self::ResourceExhausted { resource, limit, current, .. } => {
                ResourceExhaustedSnafu {
                    resource: resource.clone(),
                    limit: limit.clone(),
                    current: current.clone(),
                }.build()
            },
            Self::NotFound { resource_type, identifier, .. } => {
                NotFoundSnafu {
                    resource_type: resource_type.clone(),
                    identifier: identifier.clone(),
                }.build()
            },
            Self::StateConflict { message, .. } => {
                StateConflictSnafu {
                    message: message.clone(),
                }.build()
            },
            Self::Concurrency { message, source, .. } => {
                let cloned_source = source.as_ref().map(|s| {
                    let msg = format!("{}", s);
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)) as Box<dyn std::error::Error + Send + Sync>
                });
                ConcurrencySnafu {
                    message: message.clone(),
                    source: cloned_source,
                }.build()
            },
            Self::ExternalService { service_name, message, source, .. } => {
                let cloned_source = source.as_ref().map(|s| {
                    let msg = format!("{}", s);
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)) as Box<dyn std::error::Error + Send + Sync>
                });
                ExternalServiceSnafu {
                    service_name: service_name.clone(),
                    message: message.clone(),
                    source: cloned_source,
                }.build()
            },
            Self::MissingValue { item_description, .. } => {
                MissingValueSnafu {
                    item_description: item_description.clone(),
                }.build()
            },
            Self::MultipleErrors { errors, .. } => {
                MultipleErrorsSnafu {
                    errors: errors.clone(),
                }.build()
            },
            Self::WithRichContext { context, source, .. } => {
                WithRichContextSnafu {
                    context: context.clone(),
                    source: Box::new(source.clone()),
                }.build()
            },
            Self::Whatever { message, source, backtrace, .. } => {
                let cloned_source = source.as_ref().map(|s| {
                    let msg = format!("{}", s);
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)) as Box<dyn std::error::Error + Send + Sync>
                });
                WhateverSnafu {
                    message: message.clone(),
                    source: cloned_source,
                    backtrace: backtrace.clone(),
                }.build()
            },
        }
    }
}

impl AklypseError {
    /// Add rich context to an error
    pub fn add_context(self, context: types::ErrorContext) -> Self {
        WithRichContextSnafu {
            context,
            source: Box::new(self),
        }.build()
    }
    
    /// Add a simple message context to an error
    pub fn add_context_msg(self, message: impl Into<String>) -> Self {
        let context = types::ErrorContext::new(message);
        self.add_context(context)
    }
    
    /// Get the error category
    pub fn category(&self) -> types::ErrorCategory {
        match self {
            AklypseError::Io { .. } => types::ErrorCategory::Io,
            AklypseError::Parse { .. } => types::ErrorCategory::Parsing,
            AklypseError::Network { .. } => types::ErrorCategory::Network,
            AklypseError::Config { .. } => types::ErrorCategory::Configuration,
            AklypseError::Validation { .. } => types::ErrorCategory::Validation,
            AklypseError::Internal { .. } => types::ErrorCategory::Internal,
            AklypseError::CircuitBreakerOpen { .. } => types::ErrorCategory::CircuitBreaker,
            AklypseError::Timeout { .. } => types::ErrorCategory::Timeout,
            AklypseError::ResourceExhausted { .. } => types::ErrorCategory::ResourceExhaustion,
            AklypseError::NotFound { .. } => types::ErrorCategory::NotFound,
            AklypseError::StateConflict { .. } => types::ErrorCategory::StateConflict,
            AklypseError::Concurrency { .. } => types::ErrorCategory::Concurrency,
            AklypseError::ExternalService { .. } => types::ErrorCategory::ExternalService,
            AklypseError::MultipleErrors { .. } => types::ErrorCategory::Multiple,
            AklypseError::WithRichContext { source, .. } => source.category(),
            AklypseError::Whatever { .. } => types::ErrorCategory::Unspecified,
            AklypseError::MissingValue { .. } => types::ErrorCategory::Validation,
        }
    }
    
    /// Get the error severity
    pub fn severity(&self) -> types::ErrorSeverity {
        if let AklypseError::WithRichContext { context, .. } = self {
            context.severity
        } else {
            // Default severity for errors without explicit context
            types::ErrorSeverity::Error
        }
    }
    
    /// Get the rich context if available
    pub fn get_rich_context(&self) -> Option<&types::ErrorContext> {
        match self {
            AklypseError::WithRichContext { context, .. } => Some(context),
            _ => None,
        }
    }
}

/// Extension trait for Result to add context to an error
pub trait ResultExt<T, E_Orig> {
    /// Add a simple message context to an error
    fn context_msg(self, message: impl Into<String>) -> Result<T, AklypseError>;
    
    /// Add rich context to an error
    fn context_rich(self, context: types::ErrorContext) -> Result<T, AklypseError>;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    E: Into<AklypseError>,
{
    fn context_msg(self, message: impl Into<String>) -> Result<T, AklypseError> {
        match self {
            Ok(value) => Ok(value),
            Err(err) => {
                let err = err.into();
                Err(err.add_context_msg(message))
            }
        }
    }
    
    fn context_rich(self, context: types::ErrorContext) -> Result<T, AklypseError> {
        match self {
            Ok(value) => Ok(value),
            Err(err) => {
                let err = err.into();
                Err(err.add_context(context))
            }
        }
    }
}

/// Extension trait for Option to convert to Result with an error
pub trait OptionExt<T> {
    /// Convert Option to Result using a snafu context builder
    fn ok_or_snafu_<C, E>(self, context_builder: C) -> Result<T, E>
    where
        C: FnOnce() -> E;
    
    /// Convert Option to Result with a MissingValue error
    fn ok_or_missing_value(self, item_description: impl Into<String>) -> Result<T, AklypseError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_snafu_<C, E>(self, context_builder: C) -> Result<T, E>
    where
        C: FnOnce() -> E,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(context_builder()),
        }
    }
    
    fn ok_or_missing_value(self, item_description: impl Into<String>) -> Result<T, AklypseError> {
        match self {
            Some(v) => Ok(v),
            None => Err(MissingValueSnafu {
                item_description: item_description.into(),
            }.build()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::error::types::{ErrorContext, ErrorCategory, ErrorSource};

    #[test]
    fn test_error_creation_and_context() {
        // Create a basic error
        let err = InternalSnafu {
            message: "Test error".to_string(),
            source: None,
        }.build();

        // Verify error category
        assert_eq!(err.category(), ErrorCategory::Internal);

        // Add context to the error
        let err_with_context = err.add_context_msg("Additional context");
        
        if let AklypseError::WithRichContext { context, source, .. } = &err_with_context {
            assert_eq!(context.message, "Additional context");
            if let AklypseError::Internal { message, .. } = &**source {
                assert_eq!(message, "Test error");
            } else {
                panic!("Expected Internal error variant");
            }
        } else {
            panic!("Expected WithRichContext error variant");
        }
    }

    #[test]
    fn test_error_clone() {
        // Create an error with various fields
        let original_err = IoSnafu {
            source: Arc::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found"
            )),
            path: Some(PathBuf::from("/path/to/file")),
            operation: "read_file".to_string(),
        }.build();

        // Clone the error
        let cloned_err = original_err.clone();

        // Verify category matches
        assert_eq!(cloned_err.category(), ErrorCategory::Io);

        // Verify path and operation were properly cloned
        if let AklypseError::Io { path, operation, .. } = cloned_err {
            assert_eq!(path, Some(PathBuf::from("/path/to/file")));
            assert_eq!(operation, "read_file");
        } else {
            panic!("Expected Io error variant");
        }
    }

    #[test]
    fn test_option_ext() {
        // Test with Some value
        let opt_value: Option<i32> = Some(42);
        let result = opt_value.ok_or_missing_value("test value");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        // Test with None value
        let opt_none: Option<i32> = None;
        let result = opt_none.ok_or_missing_value("test value");
        assert!(result.is_err());
        
        if let Err(AklypseError::MissingValue { item_description, .. }) = result {
            assert_eq!(item_description, "test value");
        } else {
            panic!("Expected MissingValue error variant");
        }
    }

    #[test]
    fn test_multiple_errors() {
        // Create multiple errors
        let err1 = ValidationSnafu {
            field: "username".to_string(),
            message: "Username must be at least 3 characters".to_string(),
        }.build();
        
        let err2 = ValidationSnafu {
            field: "password".to_string(),
            message: "Password is too weak".to_string(),
        }.build();

        // Combine into MultipleErrors
        let multi_err = MultipleErrorsSnafu {
            errors: vec![err1, err2],
        }.build();

        if let AklypseError::MultipleErrors { errors, .. } = multi_err {
            assert_eq!(errors.len(), 2);
            
            if let AklypseError::Validation { field, .. } = &errors[0] {
                assert_eq!(field, "username");
            } else {
                panic!("Expected Validation error variant");
            }
            
            if let AklypseError::Validation { field, .. } = &errors[1] {
                assert_eq!(field, "password");
            } else {
                panic!("Expected Validation error variant");
            }
        } else {
            panic!("Expected MultipleErrors error variant");
        }
    }
}