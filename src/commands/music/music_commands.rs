use std::collections::VecDeque;
use lavalink_rs::model::track::{TrackData, TrackInfo};
use lavalink_rs::player_context::QueueMessage;
use lavalink_rs::prelude::{SearchEngines, TrackInQueue, TrackLoadData};
use poise::CreateReply;
use tracing::error;
use regex::Regex;
use serenity::all::CreateEmbed;
use serenity::builder::CreateEmbedAuthor;
use serenity::utils::MessageBuilder;
use songbird::error::JoinResult;
use crate::{Error, Lavalink};
use crate::commands::music::spotify;
use crate::CowContext;

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "List the commands available in the music module.")
)]
pub async fn help(ctx: CowContext<'_>) -> Result<(), Error> {
    help_code(ctx).await
}

pub async fn help_code(ctx: CowContext<'_>) -> Result<(), Error> {
    ctx.say("`help, join, leave, play, playlist, pause, now_playing, skip, queue`").await?;

    Ok(())
}

pub async fn join_interactive(ctx: &CowContext<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("Join a voice channel first.").await?;
            return Ok(());
        }
    };

    let serenity = ctx.serenity_context();
    let manager = songbird::get(serenity).await.unwrap().clone();

    let lava_client = {
        let data = serenity.data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    if lava_client.get_player_context(guild_id).is_some() {
        ctx.say("I'm already in a VC...").await?;
        return Ok(());
    }

    match manager.join_gateway(guild_id, connect_to).await {
        Ok((connection, _)) => {
            lava_client.create_player_context(guild_id, connection).await?;
            ctx.say(format!("Joined <#{connect_to}>")).await?;
        }
        Err(ex) => {
            ctx.say("Failed to connect to voice channel; maybe I don't have permissions?").await?;
            error!("Failed to connect to VC: {}", ex);
        }
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Join the voice channel you are in."),
    discard_spare_arguments
)]
pub async fn join(ctx: CowContext<'_>) -> Result<(), Error> {
    join_interactive(&ctx).await
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Make the bot leave the voice channel."),
    discard_spare_arguments
)]
pub async fn leave(ctx: CowContext<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;
    let serenity = ctx.serenity_context();

    let manager = songbird::get(serenity).await.unwrap().clone();
    let has_handler = manager.get(guild_id).is_some();

    {
        // Free up the LavaLink client.
        let data = serenity.data.read().await;
        let lava_client = data.get::<Lavalink>().unwrap().clone();
        lava_client.delete_player(guild_id.get()).await?;
    }

    if has_handler {
        if let Err(ex) = manager.remove(guild_id).await {
            error!("Failed to disconnect: {}", ex);
        }

        ctx.say("Disconnected from VC. Goodbye!").await?;
    } else {
        ctx.say("I'm not in a VC.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Play some music."),
    discard_spare_arguments
)]
pub async fn play(
    ctx: CowContext<'_>,
    #[description = "A YouTube URL or name."] #[rest] query: Option<String>)
-> Result<(), Error> {
    player_command(ctx, query, false).await
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Queue all music from a playlist."),
    discard_spare_arguments
)]
pub async fn playlist(
    ctx: CowContext<'_>,
    #[description = "A YouTube URL or query to a playlist."] #[rest] query: Option<String>)
-> Result<(), Error> {
    player_command(ctx, query, true).await
}

async fn player_command(ctx: CowContext<'_>, query: Option<String>, load_playlist: bool) -> Result<(), Error> {
    if let Some(mut query) = query {
        if let Some(guild_id) = ctx.guild_id() {
            let serenity = ctx.serenity_context();
            let lava_client = {
                let data = serenity.data.read().await;
                data.get::<Lavalink>().unwrap().clone()
            };

            let manager = songbird::get(serenity).await.unwrap().clone();

            if manager.get(guild_id).is_none() {
                if let Err(ex) = join_interactive(&ctx).await {
                    ctx.say("Failed to connect to voice channel; maybe I don't have permissions?").await?;
                    error!("Failed to connect to VC: {}", ex);
                    return Ok(());
                }
            }

            let Some(player) = lava_client.get_player_context(guild_id) else {
                ctx.say("Am I not in a VC? This shouldn't happen, disconnect me maybe?").await?;
                return Ok(());
            };

            if !query.starts_with("http") {
                // Could also use Spotify::to_query
                query = SearchEngines::YouTube.to_query(&query)?;
            }

            match lava_client.load_tracks(guild_id, &query).await {
                Ok(track) => {
                    let mut playlist_info = None;

                    let tracks: Vec<TrackInQueue> = match track.data {
                        Some(TrackLoadData::Track(x)) => vec![x.into()],
                        Some(TrackLoadData::Search(x)) => vec![x[0].clone().into()],
                        Some(TrackLoadData::Playlist(x)) => {
                            if load_playlist || x.tracks.len() == 0 {
                                playlist_info = Some(x.info);
                                x.tracks.iter().map(|x| x.into()).collect()
                            } else {
                                ctx.say("Note: only the first track will be played - use the `playlist` subcommand to load the full playlist.").await?;
                                vec![x.tracks[0].clone().into()]
                            }
                        }

                        _ => {
                            ctx.say("Could not load any tracks from the given input.").await?;
                            error!("Failed to load tracks: {:?}", track);
                            return Ok(());
                        }
                    };

                    if let Some(info) = playlist_info {
                        ctx.say(format!("Added playlist to queue: {} ({} songs)", info.name, tracks.len())).await?;
                    } else {
                        let track = &tracks[0].track;

                        if let Some(uri) = &track.info.uri {
                            ctx.say(format!("Added to queue: [{} - {}](<{}>)", track.info.author, track.info.title, uri)).await?;
                        } else {
                            ctx.say(format!("Added to queue: {} - {}", track.info.author, track.info.title)).await?;
                        }
                    }

                    player.set_queue(QueueMessage::Append(tracks.into()))?;

                    if let Ok(player_data) = player.get_player().await {
                        if player_data.track.is_none() && player.get_queue().await.is_ok_and(|x| !x.is_empty()) {
                            player.skip()?;
                        }
                    }
                }
                Err(ex) => {
                    error!("Failed to load tracks: {:?}", ex);
                    ctx.say("Could not load any tracks from the given input.").await?;
                    return Ok(());
                }
            }

            if let Some(_handler) = manager.get(guild_id) {}
        }
    } else {
        ctx.send(CreateReply::default().ephemeral(true).content("Please provide a search query.")).await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Pause the music player."),
    discard_spare_arguments
)]
pub async fn pause(ctx: CowContext<'_>) -> Result<(), Error> {
    if let Some(guild_id) = ctx.guild_id() {
        let lava_client = {
            let data = ctx.serenity_context().data.read().await;
            data.get::<Lavalink>().unwrap().clone()
        };

        if let Some(player) = lava_client.get_player_context(guild_id) {
            let paused = player.get_player().await?.paused;
            player.set_pause(!paused).await?;

            if paused {
                ctx.say("Resumed playback.").await?;
            } else {
                ctx.say("Paused playback.").await?;
            }
        } else {
            ctx.say("I'm not in a VC...").await?;
        }
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Get the current music playing."),
    aliases("np", "nowplaying"),
    discard_spare_arguments
)]
pub async fn now_playing(ctx: CowContext<'_>) -> Result<(), Error> {
    let lava_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    if let Some(node) = lava_client.get_player_context(ctx.guild_id().unwrap()) {
        if let Some(track) = &node.get_player().await?.track {
            let info = &track.info;
            let re = Regex::new(r#"(?:youtube\.com/(?:[^/]+/.+/|(?:v|e(?:mbed)?)/|.*[?&]v=)|youtu\.be/)([^"&?/\s]{11})"#).unwrap();
            let youtube_id = if let Some(uri) = &info.uri { re.captures(uri).and_then(|caps| caps.get(1).map(|m| m.as_str())) } else { None };
            let spotify_thumbail = if let Some(uri) = &info.uri { spotify::get_thumbnail(uri).await } else { None };
            let server_name = ctx.guild().map(|o| o.name.clone());

            let mut embed = CreateEmbed::new()
                .author(CreateEmbedAuthor::new(match server_name {
                    Some(name) => format!("Now Playing in {name}"),
                    None => "Now Playing".to_string()
                }))
                .title(&info.title)
                .field("Artist", &info.author, true)
                .field("Duration", format!("{}/{}", crate::util::from_ms(info.position), crate::util::from_ms(info.length)), true);

            if let Some(id) = youtube_id {
                embed = embed.thumbnail(format!("https://img.youtube.com/vi/{id}/maxresdefault.jpg"));
            } else if let Some(url) = spotify_thumbail {
                embed = embed.thumbnail(url);
            }

            if let Some(uri) = &info.uri {
                embed = embed.url(uri);
            }

            ctx.send(CreateReply::default().embed(embed)).await?;
        } else {
            ctx.say("Nothing is playing at the moment.").await?;
        }
    } else {
        ctx.say("Nothing is playing at the moment.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Skip the current song."),
    discard_spare_arguments
)]
pub async fn skip(ctx: CowContext<'_>) -> Result<(), Error> {
    let lava_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    let Some(player) = lava_client.get_player_context(ctx.guild_id().unwrap()) else {
        ctx.say("I'm not in a VC...").await?;
        return Ok(());
    };

    if let Some(track) = player.get_player().await?.track {
        player.skip()?;
        ctx.say(MessageBuilder::new().push("Skipped: ").push_mono_line_safe(&track.info.title).build()).await?;
    } else {
        ctx.say("There is nothing to skip.").await?;
    }

    Ok(())
}

fn generate_line(info: &TrackInfo) -> String {

    format!("{} - {} | ``{}``\n\n", info.title, info.author, crate::util::from_ms(info.length))
}

fn generate_queue(queue: VecDeque<TrackInQueue>) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();

    if queue.is_empty() {
        output.push("There are no songs queued.".to_string());
    }

    let mut index = 0;
    while index < queue.len() {
        let mut page = String::new();

        // Max on one page is 10 just as a hard limit
        for _ in 1..=10 {
            if index >= queue.len() {
                break;
            }

            let song = &queue[index];
            index += 1;
            let next_line = format!("``{}.`` {}", index, generate_line(&song.track.info));

            if page.len() + next_line.len() > 1024 {
                index -= 1;
                break;
            }

            page.push_str(&next_line);
        }

        output.push(page);
    }

    output
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Get the music queue."),
    aliases("q")
)]
pub async fn queue(
    ctx: CowContext<'_>,
    #[description = "The page of the queue to display"] #[min = 1] page: Option<usize>)
-> Result<(), Error> {
    let lava_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    let mut page_num = if let Some(arg_page) = page {
        arg_page
    } else {
        1
    };

    let guild_id = ctx.guild_id().unwrap();
    let Some(context) = lava_client.get_player_context(guild_id) else {
        ctx.say("Currently not connected to a voice channel.").await?;
        return Ok(());
    };

    let queue = context.get_queue().await?;
    let pages = generate_queue(queue);

    if page_num > pages.len() {
        page_num = pages.len();
    } else if page_num == 0 {
        page_num = 1;
    }

    let page = &pages[page_num - 1];
    let server_name = guild_id.name(ctx.serenity_context());

    let mut embed = CreateEmbed::new()
        .title("Now Playing")
        .field("Queued", page, false)
        .author(CreateEmbedAuthor::new(
            if let Some(server) = server_name {
                format!("Player Queue | Page {}/{} | Playing in {}", page_num, pages.len(), server)
            } else {
                format!("Player Queue | Page {}/{}", page_num, pages.len())
            }
        ));

    if let Some(now_playing) = &context.get_player().await.unwrap().track {
        embed = embed.description(generate_line(&now_playing.info));
    } else {
        embed = embed.description("Nothing is playing.");
    }

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}