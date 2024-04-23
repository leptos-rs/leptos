use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use sqlx::{PgPool, Pool, Postgres};
use std::{
    env::{self, current_dir},
    fs::File,
    io::{BufWriter, Read},
    path::Path,
};
use time::{format_description, PrimitiveDateTime};
use xml::{writer::XmlEvent, EmitterConfig, EventWriter};

#[derive(Debug)]
struct Post {
    slug: String,
    updated_at: PrimitiveDateTime,
}

/// Generates a sitemap based on data stored in a database containing slugs that we can use to build URLs to the posts themselves.
pub async fn generate_sitemap() -> impl IntoResponse {
    dotenvy::dotenv().ok();

    // Depending on your preference, we can dynamically servce the sitemap file for each,
    // or generate the file once on the first visit (probably from a bot) and write it to disk
    // so we can simply serve the created file instead of having to query the database every time
    let sitemap_path = format!(
        "{}/sitemap-index.xml",
        &current_dir().unwrap().to_str().unwrap()
    );
    let path = Path::new(&sitemap_path);

    // If the doesn't exist, we've probably deployed a fresh version of our Leptos site somewhere so we'll generate it on first request
    if !path.exists() {
        let pool = PgPool::connect(
            &env::var("DATABASE_URL").expect("database URL to exist"),
        )
        .await
        .expect("to be able to connect to pool");

        create_sitemap_file(path, pool).await.ok();
    }

    // Once the file has been written, grab the contents of it and write it out as an XML file in the response
    let mut file = File::open(sitemap_path).unwrap();
    let mut contents = vec![];
    file.read_to_end(&mut contents).ok();
    let body = Body::from(contents);

    Response::builder()
        .header("Content-Type", "application/xml")
        // Cache control can be helpful for cases where your site might be deployed occassionally and the original
        // sitemap that was generated can be cached with a header
        .header("Cache-Control", "max-age=86400")
        .body(body)
        .unwrap()
}

async fn create_sitemap_file(
    path: &Path,
    pool: Pool<Postgres>,
) -> anyhow::Result<()> {
    let file = File::create(path).expect("sitemap file to be created");
    let file = BufWriter::new(file);
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(file);

    writer
        .write(
            XmlEvent::start_element("urlset")
                .attr("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9")
                .attr("xmlns:xhtml", "http://www.w3.org/1999/xhtml")
                .attr(
                    "xmlns:image",
                    "http://www.google.com/schemas/sitemap-image/1.1",
                )
                .attr(
                    "xmlns:video",
                    "http://www.google.com/schemas/sitemap-video/1.1",
                )
                .attr(
                    "xmlns:news",
                    "http://www.google.com/schemas/sitemap-news/0.9",
                ),
        )
        .expect("xml header to be written");

    // We could also pull this from configuration or an environment variable
    let app_url = "https://mywebsite.com";

    // First, read all the blog entries so we can get the slug for building,
    //  URLs and the updated date to determine the change frequency
    sqlx::query_as!(
        Post,
        r#"
SELECT slug,
       updated_at
FROM posts
ORDER BY updated_at DESC
    "#
    )
    .fetch_all(&pool)
    .await
    .expect("")
    .into_iter()
    .try_for_each(|p| write_post_entry(p, app_url, &mut writer))?;

    // Next, write the static pages and close the XML stream
    write_static_page_entry(app_url, &mut writer)?;

    writer.write(XmlEvent::end_element())?;

    Ok(())
}

fn write_post_entry(
    post: Post,
    app_url: &str,
    writer: &mut EventWriter<BufWriter<File>>,
) -> anyhow::Result<()> {
    let format = format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second]Z",
    )?;
    let parsed_date = post.updated_at.format(&format)?;
    let route = format!("{}/blog/{}", app_url, post.slug);

    writer.write(XmlEvent::start_element("url"))?;
    writer.write(XmlEvent::start_element("loc"))?;
    writer.write(XmlEvent::characters(&route))?;
    writer.write(XmlEvent::end_element())?;
    writer.write(XmlEvent::start_element("lastmod"))?;
    writer.write(XmlEvent::characters(&parsed_date))?;
    writer.write(XmlEvent::end_element())?;
    writer.write(XmlEvent::start_element("changefreq"))?;
    writer.write(XmlEvent::characters("yearly"))?;
    writer.write(XmlEvent::end_element())?;
    writer.write(XmlEvent::start_element("priority"))?;
    writer.write(XmlEvent::characters("0.5"))?;
    writer.write(XmlEvent::end_element())?;
    writer.write(XmlEvent::end_element())?;

    Ok(())
}

fn write_static_page_entry(
    route: &str,
    writer: &mut EventWriter<BufWriter<File>>,
) -> anyhow::Result<()> {
    write_entry(route, "weekly", "0.8", writer)?;
    Ok(())
}

fn write_entry(
    route: &str,
    change_frequency: &str,
    priority: &str,
    writer: &mut EventWriter<BufWriter<File>>,
) -> anyhow::Result<()> {
    writer.write(XmlEvent::start_element("url"))?;
    writer.write(XmlEvent::start_element("loc"))?;
    writer.write(XmlEvent::characters(route))?;
    writer.write(XmlEvent::end_element())?;
    writer.write(XmlEvent::start_element("changefreq"))?;
    writer.write(XmlEvent::characters(change_frequency))?;
    writer.write(XmlEvent::end_element())?;
    writer.write(XmlEvent::start_element("priority"))?;
    writer.write(XmlEvent::characters(priority))?;
    writer.write(XmlEvent::end_element())?;
    writer.write(XmlEvent::end_element())?;

    Ok(())
}
