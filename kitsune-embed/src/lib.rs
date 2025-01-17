#![forbid(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use diesel::{OptionalExtension, QueryDsl};
use diesel_async::{pooled_connection::deadpool, RunQueryDsl};
use embed_sdk::EmbedWithExpire;
use http::{Method, Request};
use iso8601_timestamp::Timestamp;
use kitsune_db::{
    json::Json,
    model::link_preview::{ConflictLinkPreviewChangeset, LinkPreview, NewLinkPreview},
    schema::link_previews,
    PgPool,
};
use kitsune_http_client::Client as HttpClient;
use once_cell::sync::Lazy;
use scraper::{Html, Selector};
use smol_str::SmolStr;
use typed_builder::TypedBuilder;

pub use embed_sdk;
pub use embed_sdk::Embed;

type Result<T, E = Error> = std::result::Result<T, E>;

static LINK_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("a:not(.mention, .hashtag)").expect("[Bug] Failed to parse link HTML selector")
});

fn first_link_from_fragment(fragment: &str) -> Option<String> {
    let parsed_fragment = Html::parse_fragment(fragment);

    parsed_fragment
        .select(&LINK_SELECTOR)
        .next()
        .and_then(|element| element.value().attr("href").map(ToString::to_string))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    #[error(transparent)]
    Http(#[from] kitsune_http_client::Error),

    #[error(transparent)]
    Pool(#[from] deadpool::PoolError),
}

#[derive(Clone, TypedBuilder)]
pub struct Client {
    db_pool: PgPool,
    #[builder(setter(into))]
    embed_service: SmolStr,
    #[builder(default)]
    http_client: HttpClient,
}

impl Client {
    /// Fetches embed data for an HTML fragment
    ///
    /// It parses the HTML fragment, selects the first link and fetched embed data for it
    pub async fn fetch_embed_for_fragment(
        &self,
        fragment: &str,
    ) -> Result<Option<LinkPreview<Embed>>> {
        let Some(url) = first_link_from_fragment(fragment) else {
            return Ok(None);
        };
        self.fetch_embed(&url).await.map(Some)
    }

    pub async fn fetch_embed(&self, url: &str) -> Result<LinkPreview<Embed>> {
        {
            let mut db_conn = self.db_pool.get().await?;
            if let Some(data) = link_previews::table
                .find(url)
                .get_result::<LinkPreview<Embed>>(&mut db_conn)
                .await
                .optional()?
            {
                if data.expires_at > Timestamp::now_utc() {
                    return Ok(data);
                }
            }
        }

        let request = Request::builder()
            .method(Method::POST)
            .uri(self.embed_service.as_str())
            .body(url.to_string().into())
            .unwrap();

        let response = HttpClient::execute(&self.http_client, request).await?;
        let (expires_at, embed_data): EmbedWithExpire = response.json().await?;

        let mut db_conn = self.db_pool.get().await?;
        let embed_data = diesel::insert_into(link_previews::table)
            .values(NewLinkPreview {
                url,
                embed_data: Json(&embed_data),
                expires_at,
            })
            .on_conflict(link_previews::url)
            .do_update()
            .set(ConflictLinkPreviewChangeset {
                embed_data: Json(&embed_data),
                expires_at,
            })
            .get_result(&mut db_conn)
            .await?;

        Ok(embed_data)
    }
}
