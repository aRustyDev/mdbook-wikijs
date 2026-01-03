use anyhow::{Context, Result};
use mdbook::renderer::RenderContext;
use serde::Deserialize;
use std::path::PathBuf;

/// Configuration for the Wiki.js backend
/// Loaded from book.toml [output.wikijs] section
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct WikijsConfig {
    /// Output directory for Wiki.js markdown files
    pub output_dir: Option<PathBuf>,

    /// Whether to validate output against schema
    pub validate: bool,

    /// Path to custom schema file
    pub schema: Option<PathBuf>,

    /// Path prefix for Wiki.js pages (e.g., "/docs")
    pub path_prefix: Option<String>,

    /// Default frontmatter values
    pub frontmatter: FrontmatterDefaults,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct FrontmatterDefaults {
    /// Default published state
    pub published: bool,

    /// Default editor type
    pub editor: Option<String>,

    /// Default tags to add to all pages
    pub tags: Vec<String>,
}

impl Default for FrontmatterDefaults {
    fn default() -> Self {
        Self {
            published: true,
            editor: Some("markdown".to_string()),
            tags: vec![],
        }
    }
}

impl WikijsConfig {
    /// Load configuration from MDBook RenderContext
    pub fn from_render_context(ctx: &RenderContext) -> Result<Self> {
        let config = ctx
            .config
            .get("output.wikijs")
            .map(|v| {
                // Convert toml::Value -> String -> WikijsConfig via serde
                let toml_str = toml::to_string(v)
                    .context("Failed to serialize config to TOML")?;
                toml::from_str(&toml_str)
                    .context("Failed to parse [output.wikijs] configuration")
            })
            .transpose()?
            .unwrap_or_default();

        Ok(config)
    }

    /// Get the output directory, defaulting to "wikijs" in the book's build dir
    pub fn output_dir(&self, ctx: &RenderContext) -> PathBuf {
        self.output_dir
            .clone()
            .unwrap_or_else(|| ctx.destination.join("wikijs"))
    }
}
