use std::collections::HashMap;
use poise::{command, Command};
use crate::{CowContext, Error};

#[poise::command(
prefix_command,
slash_command,
description_localized("en-US", "Display the list of commands available, as well as their descriptions.")
)]
pub async fn help(
    ctx: CowContext<'_>,
    #[description = "The command requested for help"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    #[rest] command: Option<String>
) -> Result<(), Error> {
    match command {
        Some(command) => help_single_command(&ctx, &command).await,
        None => help_all_commands(&ctx).await
    }
}

async fn help_single_command<U, E>(
    ctx: &CowContext<'_>,
    command_name: &str
) -> Result<(), Error> {
    let command = ctx.framework().options().commands.iter().find(|command| {
        if command.name.eq_ignore_ascii_case(command_name) {
            return true;
        }
        if let Some(context_menu_name) = command.context_menu_name {
            if context_menu_name.eq_ignore_ascii_case(command_name) {
                return true;
            }
        }

        false
    });

    let reply = if let Some(command) = command {
        match command.help_text {
            Some(f) => f(),
            None => command
                .description
                .as_deref()
                .unwrap_or("No help available")
                .to_owned(),
        }
    } else {
        format!("No such command `{}`", command_name)
    };

    ctx.send(|b| b.content(reply))
        .await?;
    Ok(())
}

struct CommandHelp {
    prefix: String,
    name: String,
    description: String,
    subcommands: Vec<CommandHelp>
}

impl PartialEq for CommandHelp {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for CommandHelp {}

fn generate_command_help(cmd: &Command<(), Error>) -> CommandHelp {
    let description = if let Some(help_text) = cmd.help_text {
        help_text()
    } else if let Some(description) = cmd.description.as_ref() {
        description.clone()
    } else {
        "No help available".to_string()
    };

    let mut subcommands = Vec::new();

    for subcommand in &cmd.subcommands {
        subcommands.push(generate_command_help(subcommand));
    }

    CommandHelp {
        name: cmd.identifying_name.clone(),
        description,
        subcommands
    }
}

/// Code for printing an overview of all commands (e.g. `~help`)
async fn help_all_commands<U, E>(
    ctx: &CowContext<'_>
) -> Result<(), Error> {
    let mut help: Vec<CommandHelp> = Vec::new();

    let mut general = CommandHelp {
        name: "General".to_string(),
        description: "Basic commands".to_string(),
        subcommands: Vec::new()
    };

    for cmd in &ctx.framework().options().commands {
        if cmd.hide_in_help {
            continue;
        }

        let command_help = generate_command_help(cmd);

        if command_help.subcommands.is_empty() {
            general.subcommands.push(command_help);
        } else {
            help.push(command_help);
        }
    }

    ctx.send(|b| b.embed(|e| {
        e
            .title("Moogan Command Help")
            .description("You can fetch help for a specific command by passing the full command as a parameter.");

        for base_command in help {
            let command_list = base_command.subcommands.iter()
                .map(|cmd| format!("`{}`", cmd.name))
                .reduce(|a, b| format!("{}\n{}", a, b))
                .unwrap_or_default();

            e.field(base_command.name, format!("_{}_\n\n{}", base_command.description, command_list), false);
        }

        e
    })).await?;

    Ok(())
}