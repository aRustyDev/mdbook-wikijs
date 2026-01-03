use anyhow::{Context, Result};
use mdbook::book::BookItem;
use mdbook::renderer::RenderContext;
use regex::Regex;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::config::WikijsConfig;
use crate::validate::WikijsValidator;

/// Transforms MDBook chapters to Wiki.js-compatible markdown
pub struct WikijsRenderer {
    config: WikijsConfig,
    validator: Option<WikijsValidator>,
    admonition_regex: Regex,
}

impl WikijsRenderer {
    pub fn new(config: WikijsConfig, schema_path: Option<String>) -> Result<Self> {
        let validator = if config.validate || schema_path.is_some() {
            let path = schema_path
                .map(|p| Path::new(&p).to_path_buf())
                .or_else(|| config.schema.clone());
            Some(WikijsValidator::new(path)?)
        } else {
            None
        };

        // Regex to match MDBook/GitHub-style admonitions (without trailing newline for line matching)
        // Matches: > [!NOTE], > [!WARNING], > [!TIP], > [!DANGER], > [!CAUTION]
        let admonition_regex =
            Regex::new(r"^>\s*\[!(NOTE|WARNING|TIP|DANGER|CAUTION|IMPORTANT)\]\s*$")
                .context("Failed to compile admonition regex")?;

        Ok(Self {
            config,
            validator,
            admonition_regex,
        })
    }

    /// Render all chapters in the book
    pub fn render(&self, ctx: &RenderContext) -> Result<()> {
        let output_dir = self.config.output_dir(ctx);
        fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

        for item in ctx.book.iter() {
            if let BookItem::Chapter(chapter) = item {
                if chapter.path.is_some() {
                    self.render_chapter(chapter, &output_dir, ctx)?;
                }
            }
        }

        Ok(())
    }

    /// Render a single chapter
    fn render_chapter(
        &self,
        chapter: &mdbook::book::Chapter,
        output_dir: &Path,
        ctx: &RenderContext,
    ) -> Result<()> {
        let chapter_path = chapter.path.as_ref().unwrap();
        let output_path = output_dir.join(chapter_path);

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Transform the chapter content
        let transformed = self.transform_content(&chapter.content, chapter, ctx)?;

        // Generate frontmatter
        let frontmatter = self.generate_frontmatter(chapter);

        // Combine frontmatter and content
        let output = format!("---\n{}---\n\n{}", frontmatter, transformed);

        // Validate if configured
        if let Some(ref validator) = self.validator {
            validator.validate_content(&output, &chapter.name)?;
        }

        // Write to file
        let mut file = File::create(&output_path)
            .with_context(|| format!("Failed to create file: {:?}", output_path))?;
        file.write_all(output.as_bytes())?;

        log::info!("Rendered: {:?}", output_path);
        Ok(())
    }

    /// Transform MDBook markdown to Wiki.js-compatible markdown
    fn transform_content(
        &self,
        content: &str,
        _chapter: &mdbook::book::Chapter,
        _ctx: &RenderContext,
    ) -> Result<String> {
        let mut result = content.to_string();

        // Transform admonitions: > [!NOTE] -> > text\n{.is-info}
        result = self.transform_admonitions(&result);

        // Transform relative links to Wiki.js absolute paths
        result = self.transform_links(&result);

        Ok(result)
    }

    /// Transform MDBook/GitHub admonitions to Wiki.js callout syntax
    fn transform_admonitions(&self, content: &str) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Check if this line starts an admonition
            if let Some(captures) = self.admonition_regex.captures(line) {
                let admonition_type = captures.get(1).unwrap().as_str();
                let wikijs_class = match admonition_type {
                    "NOTE" | "INFO" => "{.is-info}",
                    "WARNING" | "CAUTION" => "{.is-warning}",
                    "DANGER" | "IMPORTANT" => "{.is-danger}",
                    "TIP" => "{.is-success}",
                    _ => "{.is-info}",
                };

                // Skip the admonition header line
                i += 1;

                // Collect all blockquote lines
                let mut blockquote_lines = Vec::new();
                while i < lines.len() && lines[i].starts_with('>') {
                    // Remove the leading "> " and add to blockquote
                    let content_line = lines[i].trim_start_matches('>').trim_start();
                    blockquote_lines.push(format!("> {}", content_line));
                    i += 1;
                }

                // Add the blockquote with Wiki.js class
                for line in &blockquote_lines {
                    result.push_str(line);
                    result.push('\n');
                }
                result.push_str(wikijs_class);
                result.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
                i += 1;
            }
        }

        // Remove trailing newline if original didn't have one
        if !content.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Transform relative markdown links to Wiki.js absolute paths
    fn transform_links(&self, content: &str) -> String {
        let link_regex = Regex::new(r"\[([^\]]+)\]\(\.\/([^)]+)\.md\)").unwrap();
        let prefix = self
            .config
            .path_prefix
            .as_deref()
            .unwrap_or("")
            .trim_end_matches('/');

        link_regex
            .replace_all(content, |caps: &regex::Captures| {
                let text = &caps[1];
                let path = &caps[2];
                format!("[{}]({}/{})", text, prefix, path)
            })
            .to_string()
    }

    /// Generate YAML frontmatter for a chapter
    fn generate_frontmatter(&self, chapter: &mdbook::book::Chapter) -> String {
        let mut fm = String::new();

        // Title (required)
        fm.push_str(&format!("title: \"{}\"\n", escape_yaml(&chapter.name)));

        // Published
        fm.push_str(&format!(
            "published: {}\n",
            self.config.frontmatter.published
        ));

        // Editor
        if let Some(ref editor) = self.config.frontmatter.editor {
            fm.push_str(&format!("editor: {}\n", editor));
        }

        // Tags
        if !self.config.frontmatter.tags.is_empty() {
            fm.push_str("tags:\n");
            for tag in &self.config.frontmatter.tags {
                fm.push_str(&format!("  - {}\n", tag));
            }
        }

        fm
    }
}

/// Escape special characters for YAML strings
fn escape_yaml(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admonition_transform_note() {
        let config = WikijsConfig::default();
        let renderer = WikijsRenderer::new(config, None).unwrap();

        let input = "> [!NOTE]\n> This is a note\n> with multiple lines\n";
        let expected = "> This is a note\n> with multiple lines\n{.is-info}\n";

        assert_eq!(renderer.transform_admonitions(input), expected);
    }

    #[test]
    fn test_admonition_transform_warning() {
        let config = WikijsConfig::default();
        let renderer = WikijsRenderer::new(config, None).unwrap();

        let input = "> [!WARNING]\n> Be careful!\n";
        let expected = "> Be careful!\n{.is-warning}\n";

        assert_eq!(renderer.transform_admonitions(input), expected);
    }

    #[test]
    fn test_link_transform() {
        let mut config = WikijsConfig::default();
        config.path_prefix = Some("/docs".to_string());
        let renderer = WikijsRenderer::new(config, None).unwrap();

        let input = "See [the guide](./getting-started.md) for more info.";
        let expected = "See [the guide](/docs/getting-started) for more info.";

        assert_eq!(renderer.transform_links(input), expected);
    }
}
