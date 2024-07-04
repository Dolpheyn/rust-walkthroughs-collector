mod data;
mod walkthrough_article;

use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use log::info;
use rayon::prelude::*;

use walkthrough_article::{WalkthroughArticle, WalkthroughArticlesByIssueLink};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// $HOME/.rust_walkthrough_articles
#[inline]
fn save_path() -> Result<PathBuf> {
    Ok(PathBuf::from_iter(vec![
        dirs::home_dir().ok_or("failed to get home_dir")?,
        PathBuf::from(".rust_walkthrough_articles"),
    ]))
}

fn main() -> Result<()> {
    std_logger::Config::logfmt().init();

    let local_path = save_path()?;
    let walkthrough_articles_by_issue_link = get_local_or_scrape_walkthrough_articles(&local_path)?;

    let links = walkthrough_articles_by_issue_link
        .values()
        .fold(Vec::new(), |mut acc, links| {
            acc.extend(links.iter());
            acc
        });

    let out_dir = "./output/";
    if !PathBuf::from(out_dir).exists() {
        fs::create_dir(out_dir)?;
    }

    let out_dir_scrape = format!("{}{}", out_dir, "scrape/");
    if !PathBuf::from(&out_dir_scrape).exists() {
        fs::create_dir(&out_dir_scrape)?;
    }

    let should_scrape = |_url: &str| -> bool {
        // TODO; check domain - youtube.com/youtu.be/medium.com
        false
    };

    links.par_iter().for_each(|l| {
        let link = l.link.clone();
        if !should_scrape(&link) {
            return;
        }

        let out_file = format!("{}{}", out_dir_scrape, slug::slugify(&link));
        if PathBuf::from(&out_file).exists() {
            return;
        }

        let html = data::get_page_html(&link);
        let soup = soup::Soup::new(&html);
        let mut file = File::create(out_file).unwrap();
        file.write_all(soup.text().as_bytes()).unwrap();
    });

    let scraped_files = fs::read_dir(&out_dir_scrape)?;
    for scraped_file in scraped_files.into_iter().take(1) {
        let content = fs::read(scraped_file?.path())?;
        dbg!(content);
    }

    Ok(())
}

fn print_as_markdown_list(links: &[&WalkthroughArticle]) {
    println!(
        "{}",
        links
            .iter()
            .map(|link| format!("- [{}]({})", link.title, link.link))
            .collect::<Vec<String>>()
            .join("\n")
    );
}

pub fn get_local_or_scrape_walkthrough_articles<P>(
    local_path: P,
) -> crate::Result<WalkthroughArticlesByIssueLink>
where
    P: AsRef<Path>,
{
    if let Some(from_local) = data::get_local_walkthrough_articles(&local_path)? {
        info!("[main] got walkthrough articles from local");
        return Ok(from_local);
    }

    info!("[main] start scraping");
    let res = data::scrape_walkthrough_articles_by_issue_link()?;
    if !res.is_empty() {
        data::store_locally(&local_path, &res)?;
    }

    Ok(res)
}
