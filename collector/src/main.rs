mod data;
mod walkthrough_article;

use std::{
    collections::HashSet,
    error::Error,
    fs::{self, read_dir, File},
    io::Write,
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use log::info;
use rayon::prelude::*;
use soup::prelude::*;

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

    let articles =
        walkthrough_articles_by_issue_link
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
    scrape(
        &articles
            .into_iter()
            .filter(|article| should_scrape(article))
            .take(0)
            .collect(),
        &out_dir_scrape,
    )?;

    let out_dir_contents = format!("{}{}", out_dir, "contents/");
    extract_contents(&out_dir_scrape, &out_dir_contents)?;

    Ok(())
}

fn extract_contents(scrape_dir: &str, out_dir_contents: &str) -> Result<()> {
    if !PathBuf::from(out_dir_contents).exists() {
        fs::create_dir(out_dir_contents)?;
    }

    let filenames = read_dir(scrape_dir)?
        .filter(|entry| entry.is_ok())
        .map(|entry| entry.unwrap().file_name().into_string().unwrap());

    filenames.for_each(|filename| {
        let file_content =
            String::from_utf8(fs::read(format!("{}{}", scrape_dir, &filename)).unwrap()).unwrap();
        let tags_to_find = HashSet::from(["title", "p", "ul", "ol"]);
        let tags = Soup::new(&file_content)
            .tag(true)
            .find_all()
            .filter(|t| tags_to_find.contains(t.name()));

        let content = tags
            .map(|t| t.text().trim().to_string())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if content.trim().is_empty() {
            return;
        }

        let out_file = format!("{}{}", out_dir_contents, filename);
        if PathBuf::from(&out_file).exists() {
            return;
        }

        File::create(out_file)
            .unwrap()
            .write_all(content.as_bytes())
            .unwrap();
    });

    Ok(())
}

lazy_static! {
    static ref HOSTS_TO_IGNORE: HashSet<String> = HashSet::from([
        "medium.com".to_string(),
        "www.medium.com".to_string(),
        "youtube.com".to_string(),
        "www.youtube.com".to_string(),
        "youtu.be".to_string(),
        "www.youtu.be".to_string(),
    ]);
}
fn should_scrape(article: &WalkthroughArticle) -> bool {
    if article.title.contains("[Video]") {
        return false;
    }

    let url = url::Url::parse(&article.link);
    if !url.is_ok() {
        return false;
    }
    let url = url.unwrap();

    let domain = url.domain();
    if domain.is_none() {
        return false;
    }
    let host = domain.unwrap();

    !HOSTS_TO_IGNORE.contains(host)
}

fn scrape(links: &Vec<&WalkthroughArticle>, out_dir_scrape: &str) -> Result<()> {
    if !PathBuf::from(&out_dir_scrape).exists() {
        fs::create_dir(&out_dir_scrape)?;
    }

    links.par_iter().for_each(|l| {
        let link = l.link.clone();
        let out_file = format!("{}{}", out_dir_scrape, slug::slugify(&link));
        if PathBuf::from(&out_file).exists() {
            return;
        }

        File::create(out_file)
            .unwrap()
            .write_all(data::get_page_html(&link).as_bytes())
            .unwrap();
    });

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_scrape() {
        // ignored host
        let result = should_scrape(&WalkthroughArticle {
            title: Default::default(),
            link: "https://www.youtube.com/playlist?list=PL2F_NKy2ueKOpAVPl-c3szUXuwB7K9sDq"
                .to_string(),
        });
        assert_eq!(result, false);

        // title contains "[Video]"
        let result = should_scrape(&WalkthroughArticle {
            title: "[Video] today i code rust".to_string(),
            link: Default::default(),
        });
        assert_eq!(result, false);
    }
}
