use anyhow::{bail, Context, Result};
use jsonschema::JSONSchema;
use regex::Regex;
use serde_json::Value;
use std::path::PathBuf;

/// Validates Wiki.js markdown documents
pub struct WikijsValidator {
    schema_validator: Option<JSONSchema>,
    callout_regex: Regex,
    valid_callout_classes: Vec<&'static str>,
}

impl WikijsValidator {
    /// Create a new validator, optionally loading a schema from disk
    pub fn new(schema_path: Option<PathBuf>) -> Result<Self> {
        let schema_validator = if let Some(path) = schema_path {
            let schema_str = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read schema file: {:?}", path))?;
            let schema: Value = serde_json::from_str(&schema_str)
                .context("Failed to parse schema as JSON")?;
            Some(
                JSONSchema::compile(&schema)
                    .map_err(|e| anyhow::anyhow!("Invalid JSON Schema: {}", e))?,
            )
        } else {
            None
        };

        // Regex to find Wiki.js class annotations
        let callout_regex = Regex::new(r"\{\.([a-z-]+)\}").context("Failed to compile regex")?;

        Ok(Self {
            schema_validator,
            callout_regex,
            valid_callout_classes: vec![
                "is-info",
                "is-warning",
                "is-danger",
                "is-success",
                "links-list",
                "grid-list",
                "tabset",
            ],
        })
    }

    /// Validate a complete markdown document (frontmatter + body)
    pub fn validate_content(&self, content: &str, page_name: &str) -> Result<()> {
        // Parse frontmatter
        let (frontmatter, body) = self.parse_document(content)?;

        // Layer 1: Schema validation (if schema loaded)
        if let Some(ref validator) = self.schema_validator {
            let doc = serde_json::json!({
                "frontmatter": frontmatter,
                "body": body
            });

            let result = validator.validate(&doc);
            if let Err(errors) = result {
                let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
                bail!(
                    "Schema validation failed for '{}': {}",
                    page_name,
                    error_msgs.join(", ")
                );
            }
        }

        // Layer 2: Content pattern validation
        self.validate_callout_classes(body, page_name)?;
        self.validate_frontmatter(&frontmatter, page_name)?;

        Ok(())
    }

    /// Parse a markdown document into frontmatter and body
    fn parse_document<'a>(&self, content: &'a str) -> Result<(Value, &'a str)> {
        if !content.starts_with("---\n") {
            return Ok((serde_json::json!({}), content));
        }

        let end = content[4..]
            .find("\n---\n")
            .map(|i| i + 4)
            .or_else(|| content[4..].find("\n---").map(|i| i + 4));

        match end {
            Some(end_pos) => {
                let frontmatter_str = &content[4..end_pos];
                let body_start = end_pos + 4; // Skip \n---\n
                let body = if body_start < content.len() {
                    content[body_start..].trim_start_matches('\n')
                } else {
                    ""
                };

                let frontmatter: Value = serde_yaml::from_str(frontmatter_str)
                    .context("Failed to parse frontmatter as YAML")?;

                Ok((frontmatter, body))
            }
            None => Ok((serde_json::json!({}), content)),
        }
    }

    /// Validate that only allowed Wiki.js classes are used
    fn validate_callout_classes(&self, body: &str, page_name: &str) -> Result<()> {
        for captures in self.callout_regex.captures_iter(body) {
            let class_name = captures.get(1).unwrap().as_str();
            if !self.valid_callout_classes.contains(&class_name) {
                bail!(
                    "Invalid Wiki.js class '{{.{}}}' in page '{}'. Valid classes: {}",
                    class_name,
                    page_name,
                    self.valid_callout_classes.join(", ")
                );
            }
        }
        Ok(())
    }

    /// Validate frontmatter has required fields
    fn validate_frontmatter(&self, frontmatter: &Value, page_name: &str) -> Result<()> {
        // Title is required
        if frontmatter.get("title").is_none() {
            bail!("Missing required 'title' in frontmatter for page '{}'", page_name);
        }

        // Title must be non-empty string
        if let Some(title) = frontmatter.get("title") {
            if !title.is_string() || title.as_str().unwrap_or("").is_empty() {
                bail!("'title' must be a non-empty string in page '{}'", page_name);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let validator = WikijsValidator::new(None).unwrap();
        let content = "---\ntitle: Test\npublished: true\n---\n\nBody content";

        let (fm, body) = validator.parse_document(content).unwrap();

        assert_eq!(fm.get("title").unwrap().as_str().unwrap(), "Test");
        assert_eq!(fm.get("published").unwrap().as_bool().unwrap(), true);
        assert_eq!(body, "Body content");
    }

    #[test]
    fn test_valid_callout_class() {
        let validator = WikijsValidator::new(None).unwrap();
        let body = "> This is a note\n{.is-info}\n";

        assert!(validator.validate_callout_classes(body, "test").is_ok());
    }

    #[test]
    fn test_invalid_callout_class() {
        let validator = WikijsValidator::new(None).unwrap();
        let body = "> This is invalid\n{.is-invalid}\n";

        assert!(validator.validate_callout_classes(body, "test").is_err());
    }

    #[test]
    fn test_missing_title() {
        let validator = WikijsValidator::new(None).unwrap();
        let frontmatter = serde_json::json!({
            "published": true
        });

        assert!(validator.validate_frontmatter(&frontmatter, "test").is_err());
    }

    #[test]
    fn test_valid_frontmatter() {
        let validator = WikijsValidator::new(None).unwrap();
        let frontmatter = serde_json::json!({
            "title": "My Page",
            "published": true
        });

        assert!(validator.validate_frontmatter(&frontmatter, "test").is_ok());
    }
}
