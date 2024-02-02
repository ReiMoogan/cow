use std::{borrow::Cow, io::Read};

use image::EncodableLayout;
use poise::serenity_prelude::AttachmentType;
use serenity::{model::application::component::ButtonStyle, client::Context};
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use crate::{CowContext, Error};

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Returns building floor plans"),
)]
pub async fn map(
    ctx: CowContext<'_>,
    #[description = "Building name"] building: String)
-> Result<(), Error> {

    let building_short_name = get_short_building_name(building.to_string());
    let path = format!("ucm_maps/{}", building_short_name);

    let mut entries = tokio::fs::read_dir(path).await?;
    let mut max_floors = 0;
    while let Some(_) = entries.next_entry().await? {
        max_floors += 1;
    }

    let building_short_name = get_short_building_name(building.to_string());
    let path = format!("ucm_maps/{}/floor_{}.jpg", building_short_name, 0);

    let img = tokio::fs::read(path).await?
        .bytes()
        .collect::<Result<Vec<u8>, std::io::Error>>()?;

    ctx.send(|m| {
        m.embed(|e| {
            e
                .title(format!("Map Floor of {}", building))
                .attachment("floor.jpg")
        })
        .attachment(AttachmentType::Bytes { data: Cow::from(img.as_bytes().to_owned()), filename: "floor.jpg".to_string() });
    
        if max_floors > 1 {
            m.components(|c| {
                c.create_action_row(|r| {
        
                    r.create_button(|b| {
                        let data = format!("map_next_floor:{}:{}:{}", building, 1, max_floors);
                        
                        b.style(ButtonStyle::Primary)
                        .label("Next Floor")
                        .custom_id(data)
                    })
                })
            });
        }

        m
    }).await?;

    Ok(())
}

fn get_short_building_name(building: String) -> String {
    match &building[..] {
        "campus" => "campus_map_recent",
        "cob1" => "cob",
        "glacier" => "glcr",
        "granite" => "gran",
        "library" => "kl",
        "se1" => "s_e1",
        "ssb" => "ssb_firstfloor",
        _ => &building[..]
    }.to_owned()
}

pub async fn map_next_floor(ctx: &Context, interaction: &mut MessageComponentInteraction) -> Result<(), Error> {
    let data = interaction.data.custom_id.split(':').collect::<Vec<_>>();

    let building = data[1];
    let idx = str::parse::<usize>(data[2])?;
    let floors = str::parse::<usize>(data[3])?;

    let building_short_name = get_short_building_name(building.to_string());
    let path = format!("ucm_maps/{}/floor_{}.jpg", building_short_name, idx);

    let img = tokio::fs::read(path).await?
        .bytes()
        .collect::<Result<Vec<u8>, std::io::Error>>()?;

    interaction.message.edit(ctx, |m| {
        m.embed(|e| {
            e
                .title(format!("Map Floor of {}", building))
                .attachment("floor.jpg")
        })
        .attachment(AttachmentType::Bytes { data: Cow::from(img.as_bytes().to_owned()), filename: "floor.jpg".to_string() });
    
        m.components(|c| {
            c.create_action_row(|r| {
    
                if idx > 0 {
                    r.create_button(|b| {
                        let data = format!("map_next_floor:{}:{}:{}", building, idx-1, floors);
        
                        b.style(ButtonStyle::Primary)
                            .label("Previous Floor")
                            .custom_id(data)
        
                    });
                }
    
                if idx < floors-1 {
                    r.create_button(|b| {
                        let data = format!("map_next_floor:{}:{}:{}", building, idx+1, floors);
        
                        b.style(ButtonStyle::Primary)
                            .label("Next Floor")
                            .custom_id(data)
        
                    });
                }
    
                r
            })
        })
    }).await?;

    Ok(())
}
