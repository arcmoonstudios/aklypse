/* src/common/error/types.rs */
#![warn(missing_docs)]
//! **Brief:** Core error-related structs and types for the error handling framework.
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
//! + [Error Handling Framework]
//!  - [Type Definitions]
//!  - [Error Context Structures]
//!  - [Diagnostic Utilities]
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
// **GitHub:** [ArcMoon Studios](https://github.com/arcmoonstudios)
// **Copyright:** (c) 2025 ArcMoon Studios
// **Author:** Lord Xyn
// **License:** MIT

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use std::sync::Arc;

// Note: Depending on feature flags you might use chrono::DateTime<Utc> instead of SystemTime
type TimestampType = SystemTime;

/// Severity level for errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Categorization of errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Io,
    Parsing,
    Network,
    Configuration,
    Validation,
    Internal,
    CircuitBreaker,
    Timeout,
    ResourceExhaustion,
    NotFound,
    Concurrency,
    ExternalService,
    Authentication,
    Authorization,
    StateConflict,
    Multiple,
    Unspecified,
}

/// Output formats for error reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorReportFormat {
    Plain,
    Json,
    Markdown,
    Html,
}

/// Nature of a proposed autocorrection fix
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixType {
    TextReplacement,
    AstModification,
    AddImport,
    AddDependency,
    ConfigurationChange,
    ExecuteCommand,
    Refactor,
    ManualInterventionRequired,
    Information,
    UpdateCargoToml,
    RunCargoCommand,
    SuggestAlternativeMethod,
}

/// Detailed information for specific fix types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixDetails {
    TextReplace {
        file_path: PathBuf,
        line_start: usize,
        column_start: usize,
        line_end: usize,
        column_end: usize,
        original_text_snippet: Option<String>,
        replacement_text: String,
    },
    AddImport {
        file_path: String,
        import: String,
    },
    AddCargoDependency {
        dependency: String,
        version: String,
        features: Vec<String>,
        is_dev_dependency: bool,
    },
    ExecuteCommand {
        command: String,
        args: Vec<String>,
        working_directory: Option<PathBuf>,
    },
    SuggestCodeChange {
        file_path: PathBuf,
        line_hint: usize,
        suggested_code_snippet: String,
        explanation: String,
    },
}

/// Describes the source location of an error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSource {
    pub file: String,
    pub line: u32,
    pub module_path: String,
    pub column: Option<u32>,
    pub function: Option<String>,
}

impl ErrorSource {
    pub fn new(file: impl Into<String>, line: u32, module_path: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line,
            module_path: module_path.into(),
            column: None,
            function: None,
        }
    }

    pub fn with_column(mut self, column: u32) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.function = Some(function.into());
        self
    }
}

/// Specific location for diagnostic purposes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub function_context: String,
    pub snafu_variant: Option<String>,
}

impl ErrorLocation {
    pub fn new(
        file: impl Into<String>,
        line: u32,
        column: u32,
        function_context: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            function_context: function_context.into(),
            snafu_variant: None,
        }
    }

    pub fn with_snafu_variant(mut self, variant: impl Into<String>) -> Self {
        self.snafu_variant = Some(variant.into());
        self
    }
}

/// A step in a macro expansion trace
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroExpansion {
    pub macro_name: String,
    pub expansion_site: ErrorLocation,
    pub generated_code_snippet: String,
}

/// Holds detailed diagnostic information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticResult {
    pub primary_location: Option<ErrorLocation>,
    pub expansion_trace: Vec<MacroExpansion>,
    pub suggested_fixes: Vec<String>,
    pub original_message: Option<String>,
    pub diagnostic_code: Option<String>,
}

/// Additional structured context for an error
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub message: String,
    pub source_location: Option<ErrorSource>,
    pub recovery_suggestion: Option<String>,
    pub metadata: HashMap<String, String>,
    pub severity: ErrorSeverity,
    pub timestamp: Option<TimestampType>,
    pub correlation_id: Option<String>,
    pub component: Option<String>,
    pub tags: Vec<String>,
    pub diagnostic_info: Option<DiagnosticResult>,
}

impl ErrorContext {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source_location: None,
            recovery_suggestion: None,
            metadata: HashMap::new(),
            severity: ErrorSeverity::Error,
            timestamp: Some(SystemTime::now()),
            correlation_id: None,
            component: None,
            tags: Vec::new(),
            diagnostic_info: None,
        }
    }

    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_source_location(mut self, source_location: ErrorSource) -> Self {
        self.source_location = Some(source_location);
        self
    }

    pub fn with_recovery_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.recovery_suggestion = Some(suggestion.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_component(mut self, component: impl Into<String>) -> Self {
        self.component = Some(component.into());
        self
    }

    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_diagnostic_info(mut self, diagnostic: DiagnosticResult) -> Self {
        self.diagnostic_info = Some(diagnostic);
        self
    }
}

/// A proposed autocorrection for an error
#[derive(Debug, Clone)]
pub struct Autocorrection {
    pub description: String,
    pub fix_type: FixType,
    pub confidence: f64,
    pub details: Option<FixDetails>,
    pub diff_suggestion: Option<String>,
    pub commands_to_apply: Vec<String>,
    pub targets_error_code: Option<String>,
}

impl Autocorrection {
    pub fn new(description: impl Into<String>, fix_type: FixType, confidence: f64) -> Self {
        Self {
            description: description.into(),
            fix_type,
            confidence,
            details: None,
            diff_suggestion: None,
            commands_to_apply: Vec::new(),
            targets_error_code: None,
        }
    }

    pub fn with_details(mut self, details: FixDetails) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_diff_suggestion(mut self, diff: impl Into<String>) -> Self {
        self.diff_suggestion = Some(diff.into());
        self
    }

    pub fn add_command(mut self, command: impl Into<String>) -> Self {
        self.commands_to_apply.push(command.into());
        self
    }

    pub fn with_target_error_code(mut self, code: impl Into<String>) -> Self {
        self.targets_error_code = Some(code.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Critical > ErrorSeverity::Error);
        assert!(ErrorSeverity::Error > ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning > ErrorSeverity::Info);
        assert!(ErrorSeverity::Info > ErrorSeverity::Debug);
    }

    #[test]
    fn test_error_context_building() {
        let context = ErrorContext::new("Test error")
            .with_severity(ErrorSeverity::Warning)
            .with_recovery_suggestion("Try again")
            .with_metadata("request_id", "123456")
            .with_correlation_id("corr-789")
            .with_component("auth_service")
            .add_tag("security");

        assert_eq!(context.message, "Test error");
        assert_eq!(context.severity, ErrorSeverity::Warning);
        assert_eq!(context.recovery_suggestion, Some("Try again".to_string()));
        assert_eq!(context.metadata.get("request_id"), Some(&"123456".to_string()));
        assert_eq!(context.correlation_id, Some("corr-789".to_string()));
        assert_eq!(context.component, Some("auth_service".to_string()));
        assert_eq!(context.tags.len(), 1);
        assert_eq!(context.tags[0], "security");
    }

    #[test]
    fn test_error_source() {
        let source = ErrorSource::new("src/main.rs", 42, "main")
            .with_column(10)
            .with_function("process_data");

        assert_eq!(source.file, "src/main.rs");
        assert_eq!(source.line, 42);
        assert_eq!(source.module_path, "main");
        assert_eq!(source.column, Some(10));
        assert_eq!(source.function, Some("process_data".to_string()));
    }

    #[test]
    fn test_fix_details_variants() {
        // Test TextReplace variant
        let text_replace = FixDetails::TextReplace {
            file_path: PathBuf::from("src/main.rs"),
            line_start: 10,
            column_start: 5,
            line_end: 10,
            column_end: 15,
            original_text_snippet: Some("foo(bar)".to_string()),
            replacement_text: "foo(baz)".to_string(),
        };

        // Test ExecuteCommand variant
        let exec_command = FixDetails::ExecuteCommand {
            command: "cargo".to_string(),
            args: vec!["fix".to_string(), "--allow-dirty".to_string()],
            working_directory: Some(PathBuf::from(".")),
        };

        // Test SuggestCodeChange variant
        let suggest_code = FixDetails::SuggestCodeChange {
            file_path: PathBuf::from("src/lib.rs"),
            line_hint: 42,
            suggested_code_snippet: "impl Clone for MyStruct {}".to_string(),
            explanation: "Add Clone implementation".to_string(),
        };

        // Verify they're different variants
        assert!(matches!(text_replace, FixDetails::TextReplace { .. }));
        assert!(matches!(exec_command, FixDetails::ExecuteCommand { .. }));
        assert!(matches!(suggest_code, FixDetails::SuggestCodeChange { .. }));
    }

    #[test]
    fn test_autocorrection_building() {
        let autocorrection = Autocorrection::new("Fix parse error", FixType::TextReplacement, 0.85)
            .with_details(FixDetails::TextReplace {
                file_path: PathBuf::from("src/main.rs"),
                line_start: 10,
                column_start: 5,
                line_end: 10,
                column_end: 15,
                original_text_snippet: Some("foo(bar)".to_string()),
                replacement_text: "foo(baz)".to_string(),
            })
            .with_diff_suggestion("@@ -10,5 +10,5 @@\n-foo(bar)\n+foo(baz)")
            .add_command("cargo check")
            .with_target_error_code("E0001");

        assert_eq!(autocorrection.description, "Fix parse error");
        assert_eq!(autocorrection.fix_type, FixType::TextReplacement);
        assert_eq!(autocorrection.confidence, 0.85);
        assert!(autocorrection.details.is_some());
        assert_eq!(autocorrection.diff_suggestion, Some("@@ -10,5 +10,5 @@\n-foo(bar)\n+foo(baz)".to_string()));
        assert_eq!(autocorrection.commands_to_apply.len(), 1);
        assert_eq!(autocorrection.commands_to_apply[0], "cargo check");
        assert_eq!(autocorrection.targets_error_code, Some("E0001".to_string()));
    }
}