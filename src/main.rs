use anyhow::{Context, Result};
use clap::Parser;
use mdbook::renderer::RenderContext;
use std::io;

mod config;
mod transform;
mod validate;

use config::WikijsConfig;
use transform::WikijsRenderer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Validate output against schema only (don't write files)
    #[arg(long)]
    validate_only: bool,

    /// Path to custom schema file
    #[arg(long)]
    schema: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // MDBook backends receive RenderContext via stdin as JSON
    let ctx = RenderContext::from_json(&mut io::stdin())
        .context("Failed to parse RenderContext from stdin")?;

    // Load configuration from book.toml [output.wikijs] section
    let config = WikijsConfig::from_render_context(&ctx)?;

    // Create renderer
    let renderer = WikijsRenderer::new(config, args.schema)?;

    // Render all chapters
    renderer.render(&ctx)?;

    Ok(())
}
