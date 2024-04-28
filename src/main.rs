use async_std::{
    fs::{self, File},
    io::WriteExt,
    task,
};
use clap::{command, Parser};
use reqwest::Client;
use url::Url;

use indicatif::{ProgressBar, ProgressStyle};

const MAX_POSTS_PER_REQUEST: usize = 1000;

const API_URL: &str = "https://api.rule34.xxx/index.php?page=dapi&s=post&q=index";

#[derive(Parser)]
#[command(name = "Hentai Downloader v2")]
#[command(version = "1.0")]
#[command(about = "Downloads hentai", long_about = None)]
struct Cli {
    #[arg(short, long)]
    count: usize,

    #[arg(short, long)]
    tags: String,
}

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

async fn parse_xml_file(
    url: String,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(&url).await?;

    let body = response.text().await?;

    let doc = roxmltree::Document::parse(&body)?;

    let urls = doc
        .descendants()
        .filter_map(|c| c.attribute("file_url").map(|url| url.to_string()))
        .collect();

    Ok(urls)
}

async fn download_image(
    client: Client,
    url_string: &String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = client.get(url_string).send().await?;
    let image_bytes = response.bytes().await?;

    let url = Url::parse(&url_string).unwrap();
    let filename = url.path_segments().unwrap().last().unwrap();

    let mut file = File::create(format!("hentai/{filename}")).await?;
    file.write_all(&image_bytes).await?;

    Ok(())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    fs::create_dir_all("hentai").await?;

    let urls = get_urls(cli.tags.split_whitespace().collect(), cli.count);

    let tasks = urls.into_iter().map(|url| {
        task::spawn(async move {
            let result = parse_xml_file(url).await;
            result
        })
    });

    let file_urls: Vec<String> = futures::future::try_join_all(tasks)
        .await?
        .into_iter()
        .flatten()
        .collect();

    let image_progress = ProgressBar::new(file_urls.len() as u64);
    image_progress.set_style(
        ProgressStyle::default_bar()
            .template("{wide_bar} {percent}% {pos}/{len}")
            .unwrap(),
    );

    image_progress.set_message("Downloading...");

    let client = Client::new();

    let file_tasks = file_urls.into_iter().map(|url| {
        let progress = image_progress.clone();
        let client = client.clone();
        task::spawn(async move {
            if let Err(err) = download_image(client, &url).await {
            } else {
                progress.inc(1);
            }
        })
    });

    let _ = futures::future::join_all(file_tasks).await;

    image_progress.finish_with_message("Done...");

    Ok(())
}
