use reqwest::{Client, Url};
use serde::{Serialize, Deserialize};
use tracing::error;

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoCompleteResponse {
    pub results: Vec<AutoCompletion>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoCompletion {
    pub term: String,
    pub preview: String
}

pub(crate) async fn fetch_autocomplete(query: &str) -> AutoCompleteResponse {
    let client = Client::new();
    let url = Url::parse_with_params("https://api.urbandictionary.com/v0/autocomplete-extra", &[("term", query)]);
    match url {
        Ok(url) => {
            let data = client.get(url.clone()).header("User-Agent", "Moogan/0.2.47").send().await.unwrap().text().await.unwrap();
            error!("Data: {}", data);
            error!("URL: {}", &url.as_str());

            match client.get(url).header("User-Agent", "Moogan/0.2.47").send().await {
                Ok(response) => {
                    match response.json::<AutoCompleteResponse>().await {
                        Ok(data) => {
                            data
                        }
                        Err(ex) => {
                            error!("Failed to process autocomplete: {}", ex);
                            AutoCompleteResponse {
                                results: vec![]
                            }
                        }
                    }
                }
                Err(ex) => {
                    error!("Failed to get autocomplete: {}", ex);
                    AutoCompleteResponse {
                        results: vec![]
                    }
                }
            }
        }
        Err(ex) => {
            // Silently fail
            error!("Failed to parse autocomplete URL: {}", ex);
            AutoCompleteResponse {
                results: vec![]
            }
        }
    }
}
