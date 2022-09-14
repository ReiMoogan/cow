mod general;
mod rank_config;
mod timeout;
pub mod ucm;
pub mod cowboard;
mod music;

use std::{collections::HashSet};

use serenity::{
    model::{
        id::UserId,
    }
};

use poise::Event::Message;

use crate::{CowContext, Error};
use crate::commands::general::*;
use crate::commands::rank_config::rankconfig;
use crate::commands::timeout::timeout;
use crate::commands::ucm::ucm;
use crate::commands::cowboard::cowboard;
use crate::commands::music::music;

#[poise::command(prefix_command, track_edits, slash_command)]
async fn help(
    ctx: CowContext<'_>,
    #[description = "The command requested for help"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            show_context_menu_commands: false,

            ..Default::default()
        },
    )
        .await?;
    Ok(())
}

pub async fn get_framework(pref: &str, _app_id: UserId, owners: HashSet<UserId>) -> poise::FrameworkOptions<(), Error> {
    poise::FrameworkOptions {
        commands: vec![
            info(),
            rank(),
            register(),
            disablexp(),
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
            music()
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(pref.into()),
            mention_as_prefix: true,
            ..Default::default()
        },
        listener: |ctx, event, _other_ctx, _bruh| {
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