pub mod rcon;
pub mod feed;
pub mod ping;

use rcon::*;
use feed::*;
use ping::*;

use crate::{CowContext, Error};

#[poise::command(prefix_command, slash_command,
    subcommands("ping", "feed", "rcon"),
    discard_spare_arguments,
    description_localized("en-US", "Fetch data from Minecraft servers."),
    aliases("mc"),
    identifying_name = "Minecraft"
)]
pub async fn minecraft(_ctx: CowContext<'_>) -> Result<(), Error> {
    Ok(())
}

fn parse_input(host: &str, port: Option<u16>) -> Result<String, Error> {
    let input_host: String;
    let input_port: String;

    // Check the input parameters
    let split = host.split(':').collect::<Vec<&str>>();

    if split.len() == 1 {
        input_host = split[0].to_string();
        input_port = port.unwrap_or(25565).to_string();
    } else if split.len() == 2 {
        input_host = split[0].to_string();
        input_port = split[1].to_string();
    } else {
        return Err("Invalid hostname, please try again.".into());
    }

    // Bound check our port
    let input_port = input_port.parse::<u16>();

    if input_port.is_err() {
        return Err("Invalid port, please try again.".into())
    }

    let input_port = input_port.unwrap();

    Ok(format!("{input_host}:{input_port}"))
}