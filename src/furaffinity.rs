use reqwest;
use reqwest::header::*;
use soup::prelude::*;
use soup::NodeExt;

use crate::data::AppConfiguration;

#[derive(Debug)]
pub struct SubmissionData {
    pub url: String,
    pub title: String,
    pub artist: String,
    pub date: String,
    pub tags: Vec<String>,
}

pub async fn get_submissions_site_text(data: &AppConfiguration) -> String {
    let client = reqwest::Client::new();

    let response = client
        .post("https://www.furaffinity.net/search/")
        .header(COOKIE, &data.cookies)
        .form(&data.form_payload)
        .send()
        .await
        .unwrap();

    response.text().await.unwrap()
}

pub fn parse_proto_ids(text: &str) -> Vec<i64> {
    let soup = Soup::new(text);

    soup.tag("figure")
        .find_all()
        .filter_map(|node| node.get("id"))
        .map(|text| text.replace("sid-", "").parse::<i64>().unwrap())
        .collect::<Vec<_>>()
}

pub async fn get_submission_info_text(id: i64, data: &AppConfiguration) -> String {
    let mut url = "https://www.furaffinity.net/view/".to_string();
    url.push_str(&id.to_string());

    let client = reqwest::Client::new();

    let response = client
        .get(url)
        .header(COOKIE, &data.cookies)
        .send()
        .await
        .unwrap();

    response.text().await.unwrap()
}

pub fn parse_submission_text(text: &str) -> SubmissionData {
    let soup = Soup::new(text);

    let submission_container = soup
        .class("submission-id-sub-container")
        .find()
        .unwrap()
        .display();

    let url = get_url(&text);
    let tags = get_tags(&text).unwrap_or(vec![]);

    let artist = get_artist(&submission_container);
    let title = get_title(&submission_container);
    let date = get_date(&submission_container);

    SubmissionData {
        url,
        tags,
        artist,
        title,
        date,
    }
}

fn get_tags(text: &str) -> Option<Vec<String>> {
    let soup = Soup::new(text);

    Some(
        soup.class("tags-row")
            .find()?
            .class("tags")
            .find_all()
            .map(|node| node.text())
            .collect::<Vec<_>>(),
    )
}

fn get_artist(text: &str) -> String {
    let soup = Soup::new(text);

    soup.tag("strong").find().unwrap().text()
}

fn get_title(text: &str) -> String {
    let soup = Soup::new(text);

    soup.class("submission-title")
        .find()
        .unwrap()
        .tag("p")
        .find()
        .unwrap()
        .text()
}

fn get_url(text: &str) -> String {
    let soup = Soup::new(text);

    let partial_url = soup
        .class("download")
        .find()
        .unwrap()
        .tag("a")
        .find()
        .unwrap()
        .get("href")
        .unwrap();

    format!("https:{}", partial_url)
}

fn get_date(text: &str) -> String {
    let soup = Soup::new(text);

    soup.class("popup_date")
        .find()
        .unwrap()
        .get("title")
        .unwrap()
}
