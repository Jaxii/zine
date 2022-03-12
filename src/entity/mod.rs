use anyhow::Result;
use pulldown_cmark::{html, Options, Parser as MarkdownParser};
use serde::Deserialize;
use std::{fs, path::Path};
use tera::Context;
use walkdir::WalkDir;

use crate::{zine::Render, Article, Page, Season, Theme, Zine, ZINE_FILE};

pub trait Entity {
    fn parse(&mut self, _source: &Path) -> Result<()> {
        Ok(())
    }

    fn render(&self, _context: Context, _dest: &Path) -> Result<()> {
        Ok(())
    }
}

impl<T: Entity> Entity for Vec<T> {
    fn parse(&mut self, source: &Path) -> Result<()> {
        for item in self {
            item.parse(source)?;
        }
        Ok(())
    }

    fn render(&self, render: Context, dest: &Path) -> Result<()> {
        for item in self {
            item.render(render.clone(), dest)?;
        }
        Ok(())
    }
}

impl Entity for Zine {
    fn parse(&mut self, source: &Path) -> Result<()> {
        self.theme.parse(source)?;

        self.seasons.parse(source)?;
        // Sort all seasons by number.
        self.seasons.sort_unstable_by_key(|s| s.number);

        // Parse pages
        let page_dir = source.join("pages");
        for entry in WalkDir::new(&page_dir) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let markdown = fs::read_to_string(path)?;
                let markdown_parser = MarkdownParser::new_ext(&markdown, Options::all());
                let mut html = String::new();
                html::push_html(&mut html, markdown_parser);
                self.pages.push(Page {
                    html,
                    file_path: path.strip_prefix(&page_dir)?.to_owned(),
                });
            }
        }
        Ok(())
    }

    fn render(&self, mut context: Context, dest: &Path) -> Result<()> {
        context.insert("theme", &self.theme);
        context.insert("site", &self.site);
        // Render all seasons pages.
        self.seasons.render(context.clone(), dest)?;

        // Render other pages.
        self.pages.render(context.clone(), &dest.join("page"))?;

        // Render home page.
        context.insert("seasons", &self.seasons);
        Render::render("index.jinja", &context, dest)?;
        Ok(())
    }
}

impl Entity for Theme {
    fn parse(&mut self, source: &Path) -> Result<()> {
        if let Some(footer_template) = self.footer_template.as_ref() {
            // Read footer tempolate from path to html.
            self.footer_template = Some(fs::read_to_string(source.join(&footer_template))?);
        }
        Ok(())
    }
}

impl Entity for Season {
    fn parse(&mut self, source: &Path) -> Result<()> {
        // Representing a zine.toml file for season.
        #[derive(Debug, Deserialize)]
        struct SeasonFile {
            #[serde(rename = "article")]
            articles: Vec<Article>,
        }

        let dir = source.join(&self.path);
        let content = fs::read_to_string(&dir.join(ZINE_FILE))?;
        let season_file = toml::from_str::<SeasonFile>(&content)?;
        self.articles = season_file.articles;

        self.articles.parse(&dir)?;
        Ok(())
    }

    fn render(&self, mut context: Context, dest: &Path) -> Result<()> {
        context.insert("season", &self);
        Render::render("season.jinja", &context, dest.join(&self.slug))?;
        Ok(())
    }
}

impl Entity for Article {
    fn parse(&mut self, source: &Path) -> Result<()> {
        let markdown = fs::read_to_string(&source.join(&self.file))?;
        let markdown_parser = MarkdownParser::new_ext(&markdown, Options::all());
        html::push_html(&mut self.html, markdown_parser);
        Ok(())
    }

    fn render(&self, mut context: Context, dest: &Path) -> Result<()> {
        context.insert("article", &self);
        Render::render("article.jinja", &context, dest)?;
        Ok(())
    }
}

impl Entity for Page {
    fn render(&self, mut context: Context, dest: &Path) -> Result<()> {
        context.insert("content", &self.html);
        Render::render("page.jinja", &context, dest.join(self.slug()))?;
        Ok(())
    }
}