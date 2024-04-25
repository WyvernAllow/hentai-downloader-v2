use async_std::{io::WriteExt, task};

const MAX_POSTS_PER_REQUEST: usize = 1000;

const API_URL: &str = "https://api.rule34.xxx/index.php?page=dapi&s=post&q=index";

fn get_urls(tags: Vec<&str>, count: usize) -> Vec<String> {
    let num_requests = (count + MAX_POSTS_PER_REQUEST - 1) / MAX_POSTS_PER_REQUEST;

    (0..num_requests)
        .map(move |i| {
            let offset = i * MAX_POSTS_PER_REQUEST;
            let end = usize::min((i + 1) * MAX_POSTS_PER_REQUEST, count);

            let count = end - offset;

            let tag_string = tags.join("+");

            format!("{API_URL}&limit={count}&tags={tag_string}&pid={i}")
        })
        .collect()
}

async fn download_file(
    url: String,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(&url).await?;

    println!("Requesting xml from {url}");

    let body = response.text().await?;

    println!("Parsing xml from {url}");
    let doc = roxmltree::Document::parse(&body)?;

    let urls = doc
        .descendants()
        .filter_map(|c| c.attribute("file_url").map(|url| url.to_string()))
        .collect();

    println!("Done parsing {url}");

    Ok(urls)
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let urls = get_urls(vec!["ass", "tits", "pussy"], 10100);

    let tasks = urls.into_iter().map(|url| task::spawn(download_file(url)));

    let result: Vec<String> = futures::future::try_join_all(tasks)
        .await?
        .into_iter()
        .flatten()
        .collect();
    let result_string = result.join("\n");

    let mut file = async_std::fs::File::create("output.txt").await?;
    file.write_all(result_string.as_bytes()).await?;

    Ok(())
}
