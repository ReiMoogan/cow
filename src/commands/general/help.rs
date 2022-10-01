use poise::{Command};
use serenity::utils::MessageBuilder;
use crate::{CowContext, Error};

/// Display the list of commands available, as well as their descriptions.
///
/// If you need help using help, you're truly lost.
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

async fn help_single_command(
    ctx: &CowContext<'_>,
    command_name: &str
) -> Result<(), Error> {
    let help = get_help_hierachy(ctx);
    // We will flatten the hierachy.
    let mut flattened_help = Vec::new();
    let mut iterator = help.into_iter();
    // We know the first item is all general commands, so we'll push the subcommands in there.
    flattened_help.append(&mut iterator.next().unwrap().subcommands);
    for command in iterator.by_ref() {
        flattened_help.push(command);
    }

    let input_lower = command_name.to_lowercase();
    let input = input_lower.split(' ');

    let mut command_help = CommandHelp {
        prefix: None,
        name: "".to_string(),
        description: "".to_string(),
        subcommands: flattened_help,
        aliases: Vec::new()
    };

    let safe = MessageBuilder::new()
        .push_mono_safe(command_name)
        .build();

    for command in input {
        let new_command_help = command_help.subcommands.into_iter()
            .find(|o| o.prefix.as_ref().map(|p| p.to_lowercase() == command).unwrap_or(false) || // Check for main prefix
                o.aliases.iter().map(|o| o.to_lowercase()).any(|o| o == command)); // Check for any aliases matching
        if let Some(new_command_help) = new_command_help {
            command_help = new_command_help;
        } else {
            ctx.say(format!("Command query `{}` not found.", safe)).await?;
            return Ok(());
        }
    }

    ctx.send(|m| m.embed(|e| {
        e.title(&command_help.name).description(&command_help.description);

        if let Some(prefix) = command_help.prefix.as_ref() {
            e.field("Prefix", format!("`{}`", prefix), true);
        }

        if !command_help.aliases.is_empty() {
            let aliases = command_help.aliases.iter()
                .map(|o| format!("`{}`", o))
                .reduce(|a, b| format!("{}, {}", a, b))
                .unwrap();

            e.field("Aliases", aliases, true);
        }

        e
    })).await?;

    Ok(())
}

/// Code for printing an overview of all commands (e.g. `~help`)
async fn help_all_commands(ctx: &CowContext<'_>) -> Result<(), Error> {
    let help = get_help_hierachy(ctx);

    ctx.send(|b| b.embed(|e| {
        e
            .title("Moogan Command Help")
            .description("You can fetch help for a specific command by passing the full command as a parameter.")
            .colour(0xF6DBD8);

        for base_command in help {
            let prefix = if let Some(prefix) = base_command.prefix {
                format!("\nPrefix: `{}`", prefix)
            } else {
                "".to_string()
            };

            let command_list = base_command.subcommands.iter()
                .map(|cmd| {
                    // Hope there's no sub-subcommands.
                    if cmd.subcommands.is_empty() {
                        format!("`{}`", cmd.name)
                    } else {
                        let subprefix = if let Some(prefix) = cmd.prefix.as_ref() {
                            format!("\nPrefix: `{}`", prefix)
                        } else {
                            "".to_string()
                        };

                        let subcommand_list = cmd.subcommands.iter()
                            .map(|subcmd| format!("- `{}`", subcmd.name))
                            .reduce(|a, b| format!("{}\n{}", a, b))
                            .unwrap_or_default();

                        format!("- __**{}**__{}\n\n{}", cmd.name, subprefix, subcommand_list)
                    }
                })
                .reduce(|a, b| format!("{}\n{}", a, b))
                .unwrap_or_default();

            e.field(base_command.name, format!("_{}_{}\n\n{}", base_command.description, prefix, command_list), true);
        }

        e
    })).await?;

    Ok(())
}

struct CommandHelp {
    prefix: Option<String>,
    name: String,
    description: String,
    subcommands: Vec<CommandHelp>,
    aliases: Vec<String>
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
    } else if let Some(description) = cmd.description_localizations.get("en-US") {
        description.clone()
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
        prefix: Some(cmd.name.clone()),
        name: cmd.identifying_name.clone(),
        description,
        subcommands,
        aliases: cmd.aliases.iter().map(|o| o.to_string()).collect()
    }
}

fn get_help_hierachy(ctx: &CowContext) -> Vec<CommandHelp> {
    let mut help: Vec<CommandHelp> = Vec::new();

    let mut general = CommandHelp {
        prefix: None, // There is no prefix required.
        name: "General".to_string(),
        description: "Basic commands".to_string(),
        subcommands: Vec::new(),
        aliases: Vec::new()
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

    // Insert the general group at the beginning so it appears first.
    help.insert(0, general);
    help
}