mod general;
mod rank_config;
mod timeout;
pub mod ucm;
pub mod cowboard;
mod music;

use std::{collections::HashSet};
use log::error;

use serenity::{
    model::{
        id::UserId,
        channel::Message
    },
    framework:: {
        standard::{
            macros::{
                hook,
                help
            }
            HelpOptions, CommandGroup, CommandResult, help_commands, Args, DispatchError
        }
    }
};

use crate::{CowContext, Error};

#[help]
#[individual_command_tip = "Cow help command\n\n\
Add the command you want to learn more about to the help command\n"]
#[command_not_found_text = "Could not find command: `{}`."]
#[max_levenshtein_distance(2)]
#[lacking_permissions = "Nothing"]
#[strikethrough_commands_tip_in_dm = ""]
#[strikethrough_commands_tip_in_guild = "Strikethrough commands require elevated permissions."]
async fn cow_help(
    context: &CowContext<'_>,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await?;
    Ok(())
}

#[hook]
async fn non_command(ctx: &CowContext<'_>) {
    crate::message_handler::non_command(ctx, msg).await;
}

#[hook]
async fn on_error(ctx: &CowContext<'_>, error: DispatchError, _command_name: &str) {
    if let DispatchError::Ratelimited(info) = error {
        if info.is_first_try {
            // Why round up when we can add one?
            if let Err(ex) = ctx.say(&format!("This command is rate-limited, please try this again in {} seconds.", info.as_secs() + 1)).await {
                error!("Failed to send rate-limit message: {}", ex);
            }
        }
    }
}

pub async fn get_framework(pref: &str, _app_id: UserId, owners: HashSet<UserId>) -> poise::FrameworkOptions<(), Error> {
    poise::FrameworkOptions {
        commands: vec![

        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(pref.to_string()),
            mention_as_prefix: true,
            ..Default::default()
        },
        owners,
        ..Default::default()
    }
    /*
    Arc::new(Box::new(StandardFramework::new()
        .configure(|c| c
            .prefix(pref)
            .on_mention(Some(app_id))
            .owners(owners)
        )
        .normal_message(non_command)
        .on_dispatch_error(on_error)
        .bucket("diagnostics", |b| b.limit(2).time_span(15 * 60) // 15 minute delay for scan and fix.
            .limit_for(LimitedFor::Guild)
            .await_ratelimits(0)).await  // Don't delay, force them to re-execute since we don't want to hang the bot
        .help(&COW_HELP)
        .group(&GENERAL_GROUP)
        .group(&RANKCONFIG_GROUP)
        .group(&TIMEOUT_GROUP)
        .group(&UCM_GROUP)
        .group(&COWBOARD_GROUP)
        .group(&MUSIC_GROUP)
    ))*/
}