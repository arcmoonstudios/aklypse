/* src/common/error/reporter.rs */
#![warn(missing_docs)]
//! **Brief:** Error reporting utilities for structured error displays.
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
//! + [Error Handling Framework]
//!  - [Error Reporting]
//!  - [Formatted Output]
//!  - [Diagnostic Presentation]
// ~=####====A===r===c===M===o===o===n====S===t===u===d===i===o===s====X|0|$>
// **GitHub:** [ArcMoon Studios](https://github.com/arcmoonstudios)
// **Copyright:** (c) 2025 ArcMoon Studios
// **Author:** Lord Xyn
// **License:** MIT

use super::types::{ErrorReportFormat, ErrorSeverity};
use std::io::{self, Write};

/// Configuration for the error reporter
#[derive(Debug, Clone)]
pub struct ErrorReportConfig {
    pub include_message: bool,
    pub include_source_chain: bool,
    pub include_backtrace: bool,
    pub include_rich_context: bool,
    pub include_source_location: bool,
    pub include_severity: bool,
    pub format: ErrorReportFormat,
    pub max_chain_depth: Option<usize>,
    pub pretty_print_json: bool,
    pub include_diagnostics: bool,
}

impl Default for ErrorReportConfig {
    fn default() -> Self {
        Self {
            include_message: true,
            include_source_chain: true,
            include_backtrace: true,
            include_rich_context: true,
            include_source_location: true,
            include_severity: true,
            format: ErrorReportFormat::Plain,
            max_chain_depth: None,
            pretty_print_json: true,
            include_diagnostics: true,
        }
    }
}

/// Utility for generating formatted error reports
#[derive(Debug, Default)]
pub struct ErrorReporter;

impl ErrorReporter {
    pub fn new() -> Self {
        Self
    }

    /// Report an error to a writer using the provided configuration
    pub fn report<W, E>(
        &self,
        error: &E,
        config: &ErrorReportConfig,
        writer: &mut W,
    ) -> io::Result<()>
    where
        W: Write,
        E: std::error::Error,
    {
        match config.format {
            ErrorReportFormat::Plain => self.report_plain(error, config, writer),
            ErrorReportFormat::Json => self.report_json(error, config, writer),
            ErrorReportFormat::Markdown => self.report_markdown(error, config, writer),
            ErrorReportFormat::Html => self.report_html(error, config, writer),
        }
    }

    /// Report an error as a string using the provided configuration
    pub fn report_to_string<E>(&self, error: &E, config: &ErrorReportConfig) -> String
    where
        E: std::error::Error,
    {
        let mut buffer = Vec::new();
        let _ = self.report(error, config, &mut buffer);
        String::from_utf8_lossy(&buffer).to_string()
    }

    fn report_plain<W, E>(
        &self,
        error: &E,
        config: &ErrorReportConfig,
        writer: &mut W,
    ) -> io::Result<()>
    where
        W: Write,
        E: std::error::Error,
    {
        // Implementation of plain text error reporting
        // This would use the Display or Debug implementations for errors
        // and format according to the config options
        writeln!(writer, "Error: {}", error)?;
        
        // If error supports source(), we can get the cause chain
        if config.include_source_chain {
            let mut source = error.source();
            let mut depth = 0;
            
            while let Some(err) = source {
                if let Some(max_depth) = config.max_chain_depth {
                    if depth >= max_depth {
                        writeln!(writer, "... (more causes hidden)")?;
                        break;
                    }
                }
                
                writeln!(writer, "Caused by: {}", err)?;
                source = err.source();
                depth += 1;
            }
        }
        
        // If the error has backtrace support (via ErrorCompat trait)
        // we would include it here
        
        Ok(())
    }

    fn report_json<W, E>(
        &self,
        error: &E,
        config: &ErrorReportConfig,
        writer: &mut W,
    ) -> io::Result<()>
    where
        W: Write,
        E: std::error::Error,
    {
        // Implementation of JSON error reporting would go here
        // This would serialize the error chain and related information to JSON
        writeln!(writer, "{{\"error\": \"{}\"}}", error.to_string().replace("\"", "\\\""))?;
        Ok(())
    }

    fn report_markdown<W, E>(
        &self,
        error: &E,
        config: &ErrorReportConfig,
        writer: &mut W,
    ) -> io::Result<()>
    where
        W: Write,
        E: std::error::Error,
    {
        // Implementation of Markdown error reporting would go here
        writeln!(writer, "## Error\n\n```")?;
        writeln!(writer, "{}", error)?;
        writeln!(writer, "```")?;
        Ok(())
    }    fn report_html<W, E>(
        &self,
        error: &E,
        config: &ErrorReportConfig,
        writer: &mut W,
    ) -> io::Result<()>
    where
        W: Write,
        E: std::error::Error,
    {
        // Implementation of HTML error reporting would go here
        writeln!(
            writer,
            "<div class=\"error\"><pre>{}</pre></div>",
            error.to_string().replace("<", "&lt;").replace(">", "&gt;")
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt;

    // Simple error type for testing
    #[derive(Debug)]
    struct TestError {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl Error for TestError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            self.source.as_ref().map(|s| s.as_ref() as &(dyn Error + 'static))
        }
    }

    #[test]
    fn test_error_reporter_plain_format() {
        // Create a test error
        let error = TestError {
            message: "Test error message".to_string(),
            source: None,
        };

        // Create reporter and config
        let reporter = ErrorReporter::new();
        let config = ErrorReportConfig {
            include_message: true,
            include_source_chain: true,
            include_backtrace: false,
            include_rich_context: false,
            include_source_location: false,
            include_severity: false,
            format: ErrorReportFormat::Plain,
            max_chain_depth: None,
            pretty_print_json: false,
            include_diagnostics: false,
        };

        // Generate report as string
        let report = reporter.report_to_string(&error, &config);

        // Verify report contains error message
        assert!(report.contains("Test error message"));
    }

    #[test]
    fn test_error_reporter_with_source() {
        // Create a nested error
        let source_error = TestError {
            message: "Source error".to_string(),
            source: None,
        };

        let error = TestError {
            message: "Main error".to_string(),
            source: Some(Box::new(source_error)),
        };

        // Create reporter and config
        let reporter = ErrorReporter::new();
        let config = ErrorReportConfig {
            include_message: true,
            include_source_chain: true,
            include_backtrace: false,
            include_rich_context: false,
            include_source_location: false,
            include_severity: false,
            format: ErrorReportFormat::Plain,
            max_chain_depth: None,
            pretty_print_json: false,
            include_diagnostics: false,
        };

        // Generate report as string
        let report = reporter.report_to_string(&error, &config);

        // Verify report contains both error messages
        assert!(report.contains("Main error"));
        assert!(report.contains("Source error"));
    }

    #[test]
    fn test_error_reporter_json_format() {
        // Create a test error
        let error = TestError {
            message: "JSON test error".to_string(),
            source: None,
        };

        // Create reporter and config
        let reporter = ErrorReporter::new();
        let config = ErrorReportConfig {
            format: ErrorReportFormat::Json,
            ..Default::default()
        };

        // Generate report as string
        let report = reporter.report_to_string(&error, &config);

        // Verify report is JSON formatted
        assert!(report.starts_with("{"));
        assert!(report.ends_with("}\n") || report.ends_with("}"));
        assert!(report.contains("\"error\""));
        assert!(report.contains("JSON test error"));
    }
}