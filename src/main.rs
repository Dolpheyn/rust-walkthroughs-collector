use log::info;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tl::NodeHandle;

use std::collections::HashMap;

type WalkthroughArticlesByIssueLink = HashMap<String, Vec<WalkthroughArticle>>;

fn main() {
    std_logger::Config::logfmt().init();

    let walkthrough_articles_by_issue_link =
        if let Some(from_local) = get_local_walkthrough_articles() {
            from_local
        } else {
            let res = scrape_walkthrough_articles_by_issue_link();
            if !res.is_empty() {
                store_locally(&res);
            }

            res
        };

    let _links = walkthrough_articles_by_issue_link
        .iter()
        .map(|(_, v)| v)
        .fold(Vec::new(), |mut acc, v| {
            acc.extend(v.iter());
            acc
        });
    // println!("{:#?}", links);
    // print_stats(&walkthrough_articles);
}

fn store_locally(articles: &WalkthroughArticlesByIssueLink) {
    let raw_bytes = serde_json::to_vec(articles).unwrap();
    std::fs::write("$HOME/.rust_walkthrough_articles", raw_bytes).unwrap();
    info!("stored result to $HOME/.rust_walkthrough_articles");
}

fn get_local_walkthrough_articles() -> Option<WalkthroughArticlesByIssueLink> {
    let raw_bytes = std::fs::read("$HOME/.rust_walkthrough_articles").ok();
    if raw_bytes.is_none() {
        return None;
    }

    let raw_bytes = raw_bytes.unwrap();
    let raw_str = &String::from_utf8(raw_bytes).unwrap();
    Some(serde_json::from_str(raw_str).unwrap())
}

fn scrape_walkthrough_articles_by_issue_link() -> WalkthroughArticlesByIssueLink {
    // get Past Issues page https://this-week-in-rust.org/blog/archives/index.html
    // parse into Dom
    let past_issues_page_html =
        get_page_html("https://this-week-in-rust.org/blog/archives/index.html");
    let past_issues_page_dom =
        tl::parse(&past_issues_page_html, tl::ParserOptions::default()).unwrap();

    // iterate through all issue links
    // for pages with "Rust Walkthrough" section
    let issue_links = get_all_issue_links(&past_issues_page_dom);

    let walkthrough_articles = issue_links
        .par_iter()
        .map(|issue_link| {
            info!("getting past issue - {issue_link}");
            let issue_page_html = get_page_html(&issue_link);
            let issue_page_dom = tl::parse(&issue_page_html, tl::ParserOptions::default()).unwrap();

            (issue_link, get_walkthrough_articles(&issue_page_dom))
        })
        .fold(
            || HashMap::new(),
            |mut acc, (k, v)| {
                acc.insert(k.clone(), v);
                acc
            },
        )
        .reduce(
            || HashMap::new(),
            |mut acc, m| {
                for (k, v) in m {
                    acc.insert(k, v);
                }
                acc
            },
        );

    walkthrough_articles
}

fn print_stats(articles: &WalkthroughArticlesByIssueLink) {
    let issue_with_articles = articles.par_iter().filter(|(_, v)| v.len() > 0);
    let issue_without_articles = articles.par_iter().filter(|(_, v)| v.len() == 0);

    println!(
        "issue with walkthrough section: {}",
        issue_with_articles.count()
    );
    println!(
        "issue without walkthrough section: {}",
        issue_without_articles.count()
    );
}

fn get_all_issue_links(past_issues_page_dom: &tl::VDom) -> Vec<String> {
    let dom_parser = past_issues_page_dom.parser();

    // find all `div` with class `.post-title`, which includes the link for each issues
    let mut issue_links = Vec::new();
    for div_handle in past_issues_page_dom
        .query_selector("div.post-title")
        .unwrap()
    {
        // parse div into dom
        let div_node = div_handle.get(dom_parser).unwrap();
        let div_html = div_node.inner_html(dom_parser);
        let div_dom = tl::parse(div_html.as_ref(), tl::ParserOptions::default()).unwrap();

        // find `a` in the div and get its `href` attribute's value (the link)
        // colelct into `issue_links` Vec
        let a_handle = div_dom.query_selector("a").unwrap().next().unwrap();
        let a_node = a_handle.get(div_dom.parser()).unwrap();
        match a_node {
            tl::Node::Tag(a_tag_node) => {
                let attrs = a_tag_node.attributes();
                let href = attrs.get("href").unwrap().unwrap();
                issue_links.push(href.as_utf8_str().to_string());
            }
            _ => {}
        }
    }

    issue_links
}

fn get_page_html(url: &str) -> String {
    let res = reqwest::blocking::get(url).unwrap();
    return res.text().unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
struct WalkthroughArticle {
    title: String,
    link: String,
}

fn get_walkthrough_articles(issue_page_dom: &tl::VDom) -> Vec<WalkthroughArticle> {
    let parser = issue_page_dom.parser();

    let rust_walkthroughs_title_handle = if let Some(handle) = issue_page_dom
        .query_selector("#rust-walkthroughs")
        .unwrap()
        .next()
    {
        handle
    } else {
        return Vec::new();
    };

    let walkthrough_list_handle = NodeHandle::new(rust_walkthroughs_title_handle.get_inner() + 4);
    let walkthrough_list_node = walkthrough_list_handle.get(parser).unwrap();

    let walkthrough_list_html = walkthrough_list_node.inner_html(parser);
    let walkthrough_list_dom =
        tl::parse(walkthrough_list_html.as_ref(), tl::ParserOptions::default()).unwrap();

    let list_item_handles = walkthrough_list_dom.query_selector("li").unwrap();

    let mut ret = vec![];
    for list_item_handle in list_item_handles {
        let list_item_node = list_item_handle.get(walkthrough_list_dom.parser()).unwrap();
        let list_title = list_item_node
            .inner_text(walkthrough_list_dom.parser())
            .to_string();

        let list_item_html = list_item_node.inner_html(walkthrough_list_dom.parser());
        let list_item_dom =
            tl::parse(list_item_html.as_ref(), tl::ParserOptions::default()).unwrap();

        let a_handle = if let Some(handle) = list_item_dom.query_selector("a").unwrap().next() {
            handle
        } else {
            continue;
        };

        let a_node = a_handle.get(list_item_dom.parser()).unwrap();
        let list_href = a_node
            .as_tag()
            .unwrap()
            .attributes()
            .get("href")
            .unwrap()
            .unwrap()
            .as_utf8_str()
            .to_string();

        ret.push(WalkthroughArticle {
            title: list_title,
            link: list_href,
        });
    }

    ret
}
