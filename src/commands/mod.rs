mod general;
mod rank_config;
mod timeout;
pub mod ucm;
pub mod cowboard;
mod music;
pub mod minecraft;
mod gpt;

use std::{collections::HashSet};

use serenity::{
    model::{
        id::UserId,
    }
};

use poise::Event::Message;

use crate::Error;
use crate::commands::general::*;
use crate::commands::rank_config::rankconfig;
use crate::commands::timeout::timeout;
use crate::commands::ucm::ucm;
use crate::commands::cowboard::cowboard;
use crate::commands::music::music;
use crate::commands::minecraft::*;
use crate::commands::gpt::*;

pub async fn get_framework(pref: &str, _app_id: UserId, owners: HashSet<UserId>) -> poise::FrameworkOptions<(), Error> {
    poise::FrameworkOptions {
        commands: vec![
            info(),
            rank(),
            register(),
            disablexp(),
            disableserverxp(),
            levels(),
            help(),
            bangenshinplayers(),
            banleagueplayers(),
            banvalorantplayers(),
            banoverwatchplayers(),
            rankconfig(),
            timeout(),
            ucm(),
            cowboard(),
            music(),
            reimu(),
            marisa(),
            sanae(),
            momiji(),
            reisen(),
            danbooru(),
            minecraft(),
            gpt(),
            moogan()
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(pref.into()),
            mention_as_prefix: true,
            ..Default::default()
        },
        event_handler: |ctx, event, _other_ctx, _bruh| {
            Box::pin(async move {
                if let Message { new_message } = event {
                    crate::message_handler::non_command(ctx, new_message).await?;
                }

                Ok(())
            })
        },
        owners,
        ..Default::default()
    }
}