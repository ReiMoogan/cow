pub mod ucm;

use std::{collections::HashSet};
use std::sync::Arc;
use log::error;

use serenity:: {
    model::{
        id::UserId,
        channel::Message
    },
    framework:: {
        Framework,
        standard::{
            macros::{
                hook,
                help
            },
            buckets::LimitedFor,
            StandardFramework, HelpOptions, CommandGroup, CommandResult, help_commands, Args, DispatchError
        }
    },
    client::Context
};

use crate::commands::ucm::UCM_GROUP;

#[help]
#[individual_command_tip = "Cow help command\n\n\
Add the command you want to learn more about to the help command\n"]
#[command_not_found_text = "Could not find command: `{}`."]
#[max_levenshtein_distance(2)]
#[lacking_permissions = "Nothing"]
#[strikethrough_commands_tip_in_dm = ""]
#[strikethrough_commands_tip_in_guild = "Strikethrough commands require elevated permissions."]
async fn cow_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[hook]
async fn non_command(ctx: &Context, msg: &Message) {
    crate::message_handler::non_command(ctx, msg).await;
}

pub async fn get_framework(pref: &str, app_id: UserId, owners: HashSet<UserId>) -> Arc<Box<dyn Framework + Sync + std::marker::Send>> {
    Arc::new(Box::new(StandardFramework::new()
        .configure(|c| c
            .prefix(pref)
            .on_mention(Some(app_id))
            .owners(owners)
        )
        .help(&COW_HELP)
        .group(&UCM_GROUP)
    ))
}
