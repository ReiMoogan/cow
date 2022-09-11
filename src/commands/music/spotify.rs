use tracing::error;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SpotifyEmbed {
    thumbnail_url: Option<String>
}

// Get the thumbnail for a Spotify URL.
// Will return None if the link is not from Spotify, without any queries.
pub async fn get_thumbnail(spotify_url: &str) -> Option<String> {
    if !spotify_url.contains("open.spotify.com") {
        return None;
    }

    let url = "https://embed.spotify.com/oembed/?url=".to_string() + spotify_url;
    let client = reqwest::Client::new();

    match client.get(&url).header("User-Agent", "Moogan/0.1.43").send().await {
        Ok(response) => {
            match response.json::<SpotifyEmbed>().await {
                Ok(data) => {
                    return data.thumbnail_url;
                }
                Err(ex) => {
                    error!("Failed to deserialize Spotify embed: {}", ex);
                }
            }
        }
        Err(ex) => {
            error!("Failed to get calendar: {}", ex);
        }
    }

    None
}