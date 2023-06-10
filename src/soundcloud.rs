use color_eyre::{eyre::eyre, Result};
use lazy_regex::regex_captures;
use thirtyfour::By;

use crate::AppContext;

#[instrument(skip(ctx, f))]
async fn extract_hydratable<F, T>(ctx: &AppContext<'_>, url: &str, f: F) -> Result<Option<T>>
where
    F: Fn(&str) -> Option<T>,
{
    let d = &ctx.driver;

    trace!("navigating to page");
    d.goto(url).await?;

    let scripts = d.find_all(By::Tag("script")).await?;

    trace!("found {} scripts", scripts.len());
    for script in scripts {
        let contents = script.inner_html().await?;

        if let Some(t) = f(&contents) {
            trace!("found hydratable data!");
            return Ok(Some(t));
        }
    }

    Ok(None)
}

pub async fn get_playlist_id(ctx: &AppContext<'_>, url: &str) -> Result<u64> {
    let playlist_id = extract_hydratable(ctx, url, |contents| {
        regex_captures!(
            r#""hydratable":"playlist","data":\{[a-zA-Z0-9\-_\\":,]*,"id":(\d+)"#,
            contents
        )
        .map(|(_, id)| id)
        .map(|id| id.parse::<u64>())
    })
    .await?;

    if let Some(id) = playlist_id {
        Ok(id?)
    } else {
        Err(eyre!("Could not find playlist ID"))
    }
}

pub async fn get_track_id(ctx: &AppContext<'_>, url: &str) -> Result<u64> {
    let track_id = extract_hydratable(ctx, url, |contents| {
        regex_captures!(r#""hydratable":"sound","data":\{.+?"id":(\d+)"#, contents)
            .map(|(_, id)| id)
            .map(|id| id.parse::<u64>())
    })
    .await?;

    if let Some(id) = track_id {
        Ok(id?)
    } else {
        Err(eyre!("Could not find track ID"))
    }
}
