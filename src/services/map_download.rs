use std::{error::Error, path::Path};
use scraper::{Html, Selector};
use tokio::task::JoinSet;

use pdfium_render::prelude::*;

/* Taken straight from https://docs.rs/pdfium-render/latest/pdfium_render/ */
// sadly nothing is async. pdfium dynlib bindings struct can't be shared through Arc<Mutex<T>>
// it's already guarded behind a mutex, threading cannot improve performance: https://github.com/ajrcarey/pdfium-render/blob/master/examples/thread_safe.rs
fn export_pdf_to_jpegs(path: &impl AsRef<Path>, out_path: &str, password: Option<&str>) -> Result<(), PdfiumError> {
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())?,
    );

    let document = pdfium.load_pdf_from_file(path, password)?;

    let render_config = PdfRenderConfig::new()
        .set_target_width(2000)
        .set_maximum_height(2000);

    for (index, page) in document.pages().iter().enumerate() {
        page.render_with_config(&render_config)?
            .as_image() // Renders this page to an image::DynamicImage...
            .as_rgba8() // ... then converts it to an image::Image...
            .ok_or(PdfiumError::ImageError)?
            .save_with_format(
                format!("{}/floor_{}.jpg", out_path, index),
                image::ImageFormat::Jpeg
            ) // ... and saves it to a file.
            .map_err(|_| PdfiumError::ImageError)?;
    }

    Ok(())
}

async fn get_map_urls() -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let base_url = "https://registrar.ucmerced.edu/resources/maps";
    let registrar_map_txt = reqwest::get(base_url).await?.text().await?;

    let page = Html::parse_document(&registrar_map_txt);
    let a_selector = Selector::parse("a").ok().ok_or("No <a> found")?;
    let a_elements = page.select(&a_selector);

    let urls_result: Result<Vec<String>, Box<dyn Error + Send + Sync>> = a_elements
        .map(|a_elem| {
            a_elem
                .value()
                .attr("href")
                .ok_or("no href".into())
                .map(String::from)
        })
        .collect();

    let urls = urls_result?;
    let map_urls = urls
        .iter()
        .filter(|url| url.contains("https://registrar.ucmerced.edu/sites/registrar.ucmerced.edu/files/page/documents"))
        .cloned()
        .collect::<Vec<String>>();
    
    Ok(map_urls)
}

async fn dl_all_maps_pdf() -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let map_urls = get_map_urls().await?;

    tokio::fs::create_dir_all("ucm_maps").await?;

    let mut set = JoinSet::new();

    for url in map_urls {
        set.spawn(async move {
            dl_map(&url).await
        });
    }

    let mut map_names = vec![];
    while let Some(res) = set.join_next().await {
        map_names.push(res??);
    }

    Ok(map_names)
}

async fn dl_map(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let bytes = reqwest::get(url)
        .await?
        .bytes()
        .await?;

    let name = url.split("/").last().ok_or("invalid file name")?;

    tokio::fs::write(format!("ucm_maps/{}", name), bytes).await?;

    Ok(name.to_owned())
}


pub async fn dl_and_convert_all_maps() -> Result<(), Box<dyn Error + Send + Sync>> {
    if tokio::fs::metadata("ucm_maps").await.is_ok() { // https://users.rust-lang.org/t/tokio-async-how-to-check-if-a-file-exists/80962/7
        return Ok(());
    } 

    let names = dl_all_maps_pdf().await?;

    let mut set = JoinSet::new();

    for name in names {
        set.spawn(async move {
            let dirname = name.split(".").next();
            if let None = dirname {
                return Err("bad pdf name".to_string());
            }

            let dirname = dirname.unwrap();

            if let Err(err) = tokio::fs::create_dir_all(format!("ucm_maps/{}", dirname)).await {
                return Err(err.to_string());
            }

            if let Err(err) = export_pdf_to_jpegs(&Path::new(&format!("ucm_maps/{}", name)), &format!("ucm_maps/{}", dirname), None) {
                return Err(err.to_string());
            }

            Ok(())
        });
    }

    while let Some(res) = set.join_next().await {
        let _ = res?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_dl_all_maps() {
    println!("{:?}", dl_and_convert_all_maps().await);
}