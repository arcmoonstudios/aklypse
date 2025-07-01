/* src/common/error/decrust.rs */
#![warn(missing_docs)]
//! **Brief:** Decrust autocorrection framework integration.
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
//! + [Error Handling Framework]
//!  - [Autocorrection System]
//!  - [Error Diagnostic Tools]
//!  - [Fix Suggestion Engine]
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
// **GitHub:** [ArcMoon Studios](https://github.com/arcmoonstudios)
// **Copyright:** (c) 2025 ArcMoon Studios
// **Author:** Lord Xyn
// **License:** MIT

//! This module provides the `Decrust` struct and related types for suggesting
//! potential autocorrections for errors handled by this framework.

use super::AklypseError;
use super::types::{Autocorrection, DiagnosticResult, ErrorCategory, FixDetails, FixType};
use std::path::PathBuf;
use tracing::{warn};

/// Main struct for the Decrust autocorrection capabilities.
///
/// The `Decrust` engine analyzes `AklypseError` instances to provide
/// potential automated fixes or actionable suggestions for developers.
#[derive(Debug, Default)]
pub struct Decrust {}

impl Decrust {
    /// Creates a new `Decrust` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Suggests a potential autocorrection for a given `AklypseError`.
    ///
    /// This function first checks if the error contains embedded diagnostic information
    /// with pre-suggested fixes (e.g., from a compiler or linter). If not, it falls
    /// back to suggesting fixes based on the error's category and specific variant.
    ///
    /// # Arguments
    ///
    /// * `error`: A reference to the `AklypseError` for which to suggest a fix.
    /// * `_source_code_context`: Optional context of the source code where the error occurred.
    ///   This is currently unused but is reserved for more advanced context-aware suggestions
    ///   in future versions.
    ///
    /// # Returns
    ///
    /// An `Option<Autocorrection>` containing a suggested fix, or `None` if no specific
    /// automated suggestion is available for this particular error instance.
    pub fn suggest_autocorrection(
        &self,
        error: &AklypseError,
        _source_code_context: Option<&str>, // Keep for future enhancements
    ) -> Option<Autocorrection> {
        // Prioritize fixes suggested directly by diagnostic tools if present
        if let Some(diag_info) = error.get_diagnostic_info() {
            if !diag_info.suggested_fixes.is_empty() {
                tracing::debug!("Decrust: Found tool-suggested fixes in DiagnosticResult.");
                let primary_fix_text = diag_info.suggested_fixes.join("\n");
                let file_path_from_diag = diag_info
                    .primary_location
                    .as_ref()
                    .map(|loc| PathBuf::from(&loc.file));

                let details = file_path_from_diag.map(|fp| FixDetails::TextReplace {
                    file_path: fp,
                    line_start: diag_info.primary_location.as_ref().map_or(0, |loc| loc.line as usize),
                    column_start: diag_info.primary_location.as_ref().map_or(0, |loc| loc.column as usize),
                    line_end: diag_info.primary_location.as_ref().map_or(0, |loc| loc.line as usize),
                    column_end: diag_info.primary_location.as_ref().map_or(0, |loc| {
                        loc.column as usize + primary_fix_text.chars().filter(|&c| c != '\n').count().max(1)
                    }),
                    original_text_snippet: diag_info.original_message.clone(),
                    replacement_text: primary_fix_text,
                });

                return Some(Autocorrection {
                    description: "Apply fix suggested by diagnostic tool.".to_string(),
                    fix_type: FixType::TextReplacement,
                    confidence: 0.85, // High confidence for tool-provided suggestions
                    details,
                    diff_suggestion: None, // Could be generated
                    commands_to_apply: vec![],
                    targets_error_code: diag_info.diagnostic_code.clone(),
                });
            }
        }

        // Fallback to general error category based suggestions
        match error.category() {
            ErrorCategory::NotFound => {
                let (resource_type, identifier) = if let AklypseError::NotFound { resource_type, identifier, .. } = error {
                    (resource_type.clone(), identifier.clone())
                } else {
                    // Should not happen if category matches variant, but good for robustness
                    tracing::warn!("Decrust: NotFound category with unexpected error variant: {:?}", error);
                    ("unknown resource".to_string(), "unknown identifier".to_string())
                };

                let mut commands = vec![];
                let mut suggestion_details = None;
                if resource_type == "file" || resource_type == "path" {
                    let path_buf = PathBuf::from(&identifier);
                    if let Some(parent) = path_buf.parent() {
                        if !parent.as_os_str().is_empty() && !parent.exists() { // Check if parent needs creation
                            commands.push(format!("mkdir -p \"{}\"", parent.display()));
                        }
                    }
                    commands.push(format!("touch \"{}\"", identifier));
                    suggestion_details = Some(FixDetails::ExecuteCommand {
                        command: commands.first().cloned().unwrap_or_default(), // Simplified, could be multiple
                        args: commands.iter().skip(1).cloned().collect(),
                        working_directory: None,
                    });
                }
                Some(Autocorrection {
                    description: format!(
                        "Resource type '{}' with identifier '{}' not found. Consider creating it if it's a file/directory, or verify the path/name.",
                        resource_type, identifier
                    ),
                    fix_type: if commands.is_empty() { FixType::ManualInterventionRequired } else { FixType::ExecuteCommand },
                    confidence: 0.7,
                    details: suggestion_details,
                    diff_suggestion: None,
                    commands_to_apply: commands,
                    targets_error_code: Some(format!("{:?}", ErrorCategory::NotFound)),
                })
            }
            ErrorCategory::Io => {
                let (source_msg, path_opt, operation_opt, io_kind_opt) = if let AklypseError::Io { source, path, operation, .. } = error {
                    (source.to_string(), path.clone(), Some(operation.clone()), Some(source.kind()))
                } else {
                    (String::from("Unknown I/O error"), None, None, None)
                };
                let path_str = path_opt.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<unknown_path>".to_string());
                let op_str = operation_opt.unwrap_or_else(|| "<unknown_op>".to_string());

                let mut details = None;
                let mut commands = vec![];
                let fix_type = match io_kind_opt {
                    Some(std::io::ErrorKind::NotFound) => {
                        if let Some(p) = &path_opt {
                            details = Some(FixDetails::SuggestCodeChange {
                                file_path: p.clone(),
                                line_hint: 0, // Placeholder, context would improve this
                                suggested_code_snippet: format!("// Ensure path '{}' exists before operation '{}'\n// Or handle the NotFound error gracefully.", p.display(), op_str),
                                explanation: "The file or directory specified in the operation was not found at the given path.".to_string(),
                            });
                            if p.is_dir() || p.extension().is_none() { // Heuristic for directory
                                commands.push(format!("mkdir -p \"{}\"", p.display()));
                            } else { // Likely a file
                                 if let Some(parent) = p.parent() {
                                     if !parent.as_os_str().is_empty() && !parent.exists() {
                                         commands.push(format!("mkdir -p \"{}\"", parent.display()));
                                     }
                                 }
                                 commands.push(format!("touch \"{}\"", p.display()));
                            }
                        }
                        FixType::ExecuteCommand // With commands, or ManualInterventionRequired if no commands
                    }
                    Some(std::io::ErrorKind::PermissionDenied) => {
                        details = Some(FixDetails::SuggestCodeChange{
                            file_path: path_opt.clone().unwrap_or_else(|| PathBuf::from("unknown_file_causing_permission_error")),
                            line_hint: 0,
                            suggested_code_snippet: format!("// Check permissions for path '{}' for operation '{}'", path_str, op_str),
                            explanation: "The application does not have the necessary permissions to perform the I/O operation.".to_string()
                        });
                        FixType::ConfigurationChange // e.g., chmod, chown
                    }
                    _ => FixType::Information,
                };

                Some(Autocorrection {
                    description: format!("I/O error during '{}' on path '{}': {}. Verify path, permissions, or disk space.", op_str, path_str, source_msg),
                    fix_type,
                    confidence: 0.65,
                    details,
                    diff_suggestion: None,
                    commands_to_apply: commands,
                    targets_error_code: Some(format!("{:?}", ErrorCategory::Io)),
                })
            }
            ErrorCategory::Configuration => {
                let (message, path_opt) = if let AklypseError::Config { message, path, .. } = error {
                    (message.clone(), path.clone())
                } else {
                    ("Unknown configuration error".to_string(), None)
                };
                let target_file = path_opt.clone().unwrap_or_else(|| PathBuf::from("config.toml")); // Default assumption
                Some(Autocorrection {
                    description: format!("Configuration issue for path '{}': {}. Please review the configuration file structure and values.",
                        path_opt.as_ref().map(|p| p.display().to_string()).unwrap_or_else(||"<unknown_config>".to_string()), message),
                    fix_type: FixType::ConfigurationChange,
                    confidence: 0.7,
                    details: Some(FixDetails::SuggestCodeChange {
                        file_path: target_file,
                        line_hint: 1, // Suggest reviewing start of file
                        suggested_code_snippet: format!("# Review this configuration file for error related to: {}\n# Ensure all values are correctly formatted and all required fields are present.", message),
                        explanation: "Configuration files require specific syntax, valid values, and all mandatory fields to be present.".to_string()
                    }),
                    diff_suggestion: None,
                    commands_to_apply: vec![],
                    targets_error_code: Some(format!("{:?}", ErrorCategory::Configuration)),
                })
            }
            // Further specific category handling can be added here
            _ => {
                tracing::trace!(
                    "Decrust: No specific autocorrection implemented for error category: {:?}. Error: {}",
                    error.category(), error
                );
                None
            }
        }
    }
}

/// Trait to extend error types with autocorrection capabilities.
///
/// This trait should be implemented for the main error type of the application (`AklypseError`)
/// to enable the Decrust engine to provide suggestions.
pub trait AutocorrectableError {
    /// Suggests a potential autocorrection for this error.
    ///
    /// # Arguments
    /// * `decrust_engine`: An instance of the `Decrust` engine to generate suggestions.
    /// * `source_code_context`: Optional string slice containing the source code
    ///   around where the error might have originated, for more context-aware suggestions.
    fn suggest_autocorrection(
        &self,
        decrust_engine: &Decrust,
        source_code_context: Option<&str>,
    ) -> Option<Autocorrection>;

    /// Retrieves diagnostic information if available within the error structure.
    /// This is useful if the error originated from a tool (like a compiler or linter)
    /// that provides structured diagnostic output.
    fn get_diagnostic_info(&self) -> Option<&DiagnosticResult>;
}

/// Implementation of AutocorrectableError for AklypseError
///
/// This implementation enables the Aklypse error system to provide intelligent 
/// autocorrection suggestions for errors that occur during application execution.
/// It integrates with the Decrust engine to analyze errors and suggest potential fixes.
///
/// The implementation:
/// 1. Delegates autocorrection suggestion to the Decrust engine
/// 2. Accesses diagnostic information embedded within rich error contexts
/// 3. Supports the Diamond certification requirements for comprehensive error handling
impl AutocorrectableError for super::AklypseError {
    /// Suggests a possible autocorrection for this error using the Decrust engine.
    ///
    /// # Arguments
    ///
    /// * `decrust_engine` - The Decrust engine instance that will analyze the error
    /// * `source_code_context` - Optional source code context that may help with generating more accurate suggestions
    ///
    /// # Returns
    ///
    /// An `Option<Autocorrection>` containing a suggested fix, or `None` if no fix can be suggested
    fn suggest_autocorrection(
        &self,
        decrust_engine: &Decrust,
        source_code_context: Option<&str>,
    ) -> Option<Autocorrection> {
        decrust_engine.suggest_autocorrection(self, source_code_context)
    }

    /// Retrieves diagnostic information embedded within the error if available.
    ///
    /// This method looks for diagnostic information in errors that contain rich context,
    /// which may have been generated by compilers, linters, or other diagnostic tools.
    ///
    /// # Returns
    ///
    /// An `Option<&DiagnosticResult>` containing diagnostic information, or `None` if no such information exists
    fn get_diagnostic_info(&self) -> Option<&DiagnosticResult> {
        if let super::AklypseError::WithRichContext { context, .. } = self {
            context.diagnostic_info.as_ref()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{AklypseError, IoSnafu, NotFoundSnafu, WithRichContextSnafu};
    use crate::error::types::{DiagnosticResult, ErrorContext, ErrorLocation, FixType};
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn test_decrust_suggest_autocorrection_for_notfound() {
        // Create a Decrust engine
        let decrust = Decrust::new();
        
        // Create a NotFound error
        let error = NotFoundSnafu {
            resource_type: "file".to_string(),
            identifier: "/path/to/missing_file.txt".to_string(),
        }.build();
        
        // Use the error via the AutocorrectableError trait
        let autocorrection = error.suggest_autocorrection(&decrust, None);
        
        // Verify the autocorrection
        assert!(autocorrection.is_some(), "Expected autocorrection for NotFound error");
        
        if let Some(correction) = autocorrection {
            assert_eq!(correction.fix_type, FixType::ExecuteCommand);
            assert!(correction.description.contains("Resource type 'file'"));
            assert!(correction.description.contains("/path/to/missing_file.txt"));
            assert!(correction.commands_to_apply.iter().any(|cmd| cmd.contains("touch")));
        }
    }
    
    #[test]
    fn test_decrust_get_diagnostic_info() {
        // Create a diagnostic result
        let diagnostic = DiagnosticResult {
            primary_location: Some(ErrorLocation::new(
                "src/main.rs", 42, 10, "main"
            )),
            expansion_trace: Vec::new(),
            suggested_fixes: vec!["Replace `foo` with `bar`".to_string()],
            original_message: Some("Invalid syntax".to_string()),
            diagnostic_code: Some("E0001".to_string()),
        };
        
        // Create context with the diagnostic info
        let context = ErrorContext::new("Error with diagnostic info")
            .with_diagnostic_info(diagnostic);
        
        // Create a base error
        let base_error = IoSnafu {
            source: Arc::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid data"
            )),
            path: Some(PathBuf::from("src/main.rs")),
            operation: "parse".to_string(),
        }.build();
        
        // Add rich context with diagnostic info
        let error = WithRichContextSnafu {
            context,
            source: Box::new(base_error),
        }.build();
        
        // Get diagnostic info via the trait
        let diagnostic_info = error.get_diagnostic_info();
        
        // Verify diagnostic info
        assert!(diagnostic_info.is_some(), "Expected diagnostic info");
        
        if let Some(info) = diagnostic_info {
            assert_eq!(info.suggested_fixes.len(), 1);
            assert_eq!(info.suggested_fixes[0], "Replace `foo` with `bar`");
            assert_eq!(info.diagnostic_code, Some("E0001".to_string()));
            
            if let Some(location) = &info.primary_location {
                assert_eq!(location.file, "src/main.rs");
                assert_eq!(location.line, 42);
            } else {
                panic!("Expected primary location in diagnostic info");
            }
        }
    }

    #[test]
    fn test_autocorrection_for_embedded_diagnostic() {
        // Create a Decrust engine
        let decrust = Decrust::new();
        
        // Create a diagnostic result with suggested fixes
        let diagnostic = DiagnosticResult {
            primary_location: Some(ErrorLocation::new(
                "src/main.rs", 42, 10, "main"
            )),
            expansion_trace: Vec::new(),
            suggested_fixes: vec!["Fix: add semicolon".to_string()],
            original_message: Some("Missing semicolon".to_string()),
            diagnostic_code: Some("E0001".to_string()),
        };
        
        // Create context with the diagnostic info
        let context = ErrorContext::new("Syntax error")
            .with_diagnostic_info(diagnostic);
        
        // Create a base error
        let base_error = IoSnafu {
            source: Arc::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Parser error"
            )),
            path: Some(PathBuf::from("src/main.rs")),
            operation: "parse".to_string(),
        }.build();
        
        // Add rich context with diagnostic info
        let error = WithRichContextSnafu {
            context,
            source: Box::new(base_error),
        }.build();
        
        // Get autocorrection via the trait
        let autocorrection = error.suggest_autocorrection(&decrust, None);
        
        // Verify autocorrection uses diagnostic info
        assert!(autocorrection.is_some(), "Expected autocorrection from diagnostic info");
        
        if let Some(correction) = autocorrection {
            assert_eq!(correction.fix_type, FixType::TextReplacement);
            assert!(correction.description.contains("Apply fix suggested by diagnostic tool"));
            assert_eq!(correction.targets_error_code, Some("E0001".to_string()));
        }
    }
}