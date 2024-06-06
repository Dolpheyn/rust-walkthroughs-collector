mod data;
mod walkthrough_article;

use std::path::{Path, PathBuf};

use log::info;

use walkthrough_article::{WalkthroughArticle, WalkthroughArticlesByIssueLink};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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

    print_as_markdown_list(&links);

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
