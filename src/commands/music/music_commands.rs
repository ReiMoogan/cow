use lavalink_rs::model::{TrackQueue};
use tracing::error;
use regex::Regex;
use serenity::utils::MessageBuilder;
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

    let (_, handler) = manager.join_gateway(guild_id, connect_to).await;

    match handler {
        Ok(connection_info) => {
            let lava_client = {
                let data = serenity.data.read().await;
                data.get::<Lavalink>().unwrap().clone()
            };

            lava_client.create_session_with_songbird(&connection_info).await?;
            ctx.say(format!("Joined <#{connect_to}>")).await?;
        }
        Err(ex) => {
            ctx.say("Failed to join your VC...").await?;
            error!("Error joining the channel: {}", ex)
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

    if has_handler {
        if let Err(ex) = manager.remove(guild_id).await {
            error!("Failed to disconnect: {}", ex);
        }

        {
            // Free up the LavaLink client.
            let data = serenity.data.read().await;
            let lava_client = data.get::<Lavalink>().unwrap().clone();
            lava_client.destroy(guild_id.0).await?;
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
    if let Some(query) = query {
        let guild_id = match ctx.guild_id() {
            Some(channel) => channel,
            None => {
                ctx.say("Error finding channel info").await?;
                return Ok(());
            }
        };

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

        if let Some(_handler) = manager.get(guild_id) {
            let query_information = lava_client.auto_search_tracks(&query).await?;

            if query_information.tracks.is_empty() {
                ctx.say("Could not find any video of the search query.").await?;
                return Ok(());
            }

            if let Err(why) = &lava_client.play(guild_id.0, query_information.tracks[0].clone()).queue()
                .await
            {
                error!("Failed to queue: {}", why);
                return Ok(());
            };

            let message = MessageBuilder::new().push("Added to queue: ").push_mono_safe(&query_information.tracks[0].info.as_ref().unwrap().title).build();
            if let Ok(tracks) = lava_client.get_tracks(query).await {
                if tracks.tracks.len() > 1 {
                    ctx.say("Note: This seems to be a playlist. If you want to add all tracks at once, use `playlist` instead of `play`.\n".to_string() + &message).await?;
                    return Ok(())
                }
            }
            ctx.say(message).await?;
        }
    } else {
        ctx.send(|msg| msg.ephemeral(true).content("Please provide a search query.")).await?;
    }

    Ok(())
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
    if let Some(query) = query {
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

            if let Some(_handler) = manager.get(guild_id) {
                match lava_client.get_tracks(&query).await {
                    Ok(tracks) => {
                        for track in &tracks.tracks {
                            if let Err(why) = &lava_client.play(guild_id, track.clone()).queue()
                                .await
                            {
                                error!("Failed to queue from playlist: {}", why);
                            };
                        }

                        if let Some(info) = &tracks.playlist_info {
                            if let Some(name) = &info.name {
                                ctx.say(MessageBuilder::new().push("Added to the queue ").push(tracks.tracks.len()).push(" tracks from ").push_mono_safe(name).push(".").build()).await?;
                            } else {
                                ctx.say(format!("Added to the queue {} tracks.", tracks.tracks.len())).await?;
                            }
                        } else {
                            ctx.say(format!("Added to the queue {} tracks.", tracks.tracks.len())).await?;
                        }
                    }
                    Err(ex) => {
                        error!("Failed to load tracks: {}", ex);
                        ctx.say("Could not load any tracks from the given input.").await?;
                    }
                }
            }
        }
    } else {
        ctx.send(|msg| msg.ephemeral(true).content("Please provide a search query.")).await?;
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

        if let Some(node) = lava_client.nodes().await.get(&guild_id.0) {
            if node.is_paused {
                if let Err(ex) = lava_client.set_pause(guild_id.0, false).await {
                    error!("Failed to unpause music: {}", ex);
                } else {
                    ctx.say("Unpaused the player.").await?;
                }
            } else if let Err(ex) = lava_client.pause(guild_id.0).await {
                error!("Failed to pause music: {}", ex);
            } else {
                ctx.say("Paused the player.").await?;
            }
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

    if let Some(node) = lava_client.nodes().await.get(&ctx.guild_id().unwrap().0) {
        if let Some(track) = &node.now_playing {
            let info = track.track.info.as_ref().unwrap();
            let re = Regex::new(r#"(?:youtube\.com/(?:[^/]+/.+/|(?:v|e(?:mbed)?)/|.*[?&]v=)|youtu\.be/)([^"&?/\s]{11})"#).unwrap();
            let youtube_id = re.captures(&info.uri).and_then(|caps| caps.get(1).map(|m| m.as_str()));
            let spotify_thumbail = spotify::get_thumbnail(&info.uri).await;
            let server_name = ctx.guild().map(|o| o.name);

            ctx.send(|m| {
                m.embeds.clear();
                m.embed(|e| {
                    e
                        .author(|a| a.name(match server_name {
                            Some(name) => format!("Now Playing in {name}"),
                            None => "Now Playing".to_string()
                        }))
                        .title(&info.title)
                        .url(&info.uri)
                        .field("Artist", &info.author, true)
                        .field("Duration", format!("{}/{}", crate::util::from_ms(info.position), crate::util::from_ms(info.length)), true);


                    if let Some(requester) = track.requester {
                        e.field("Requested By", format!("<@{requester}>"), true);
                    }

                    if let Some(id) = youtube_id {
                        e.thumbnail(format!("https://img.youtube.com/vi/{id}/maxresdefault.jpg"));
                    } else if let Some(url) = spotify_thumbail {
                        e.thumbnail(url);
                    }

                    e
                }
                )
            }).await?;
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

    if let Some(track) = lava_client.skip(ctx.guild_id().unwrap()).await {
        ctx.say(MessageBuilder::new().push("Skipped: ").push_mono_line_safe(&track.track.info.as_ref().unwrap().title).build()).await?;

        // Need to check if it's empty, so we can stop playing (can crash if we don't check)
        if let Some(node) = lava_client.nodes().await.get(&ctx.guild_id().unwrap().0) {
            if node.now_playing.is_none() {
                if let Err(ex) = lava_client.stop(ctx.guild_id().unwrap()).await {
                    error!("Failed to stop music: {}", ex);
                }
            }
        }
    } else {
        ctx.say("There is nothing to skip.").await?;
    }

    Ok(())
}

fn generate_line(song: &TrackQueue) -> String {
    let info = song.track.info.as_ref().unwrap();

    if let Some(person) = song.requester {
        format!("{} - {} | ``{}`` Requested by: <@{}>\n\n", info.title, info.author, crate::util::from_ms(info.length), person)
    } else {
        format!("{} - {} | ``{}``\n\n", info.title, info.author, crate::util::from_ms(info.length))
    }
}

fn generate_queue(queue: &[TrackQueue]) -> Vec<String> {
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
            let next_line = format!("``{}.`` {}", index, generate_line(song));

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
    if let Some(node) = lava_client.nodes().await.get(&guild_id.0) {
        let queue = &node.queue;
        let pages = generate_queue(queue);

        if page_num > pages.len() {
            page_num = pages.len();
        } else if page_num == 0 {
            page_num = 1;
        }

        let page = &pages[page_num - 1];
        let server_name = guild_id.name(ctx.serenity_context());

        ctx.send(|m| {
            m.embeds.clear();
            m.embed(|e| {
                e
                    .author(|a| {
                        if let Some(server) = server_name {
                            a.name(format!("Player Queue | Page {}/{} | Playing in {}", page_num, pages.len(), server));
                        } else {
                            a.name(format!("Player Queue | Page {}/{}", page_num, pages.len()));
                        }

                        a
                    })
                    .title("Now Playing")
                    .field("Queued", page, false);

                if let Some(now_playing) = &node.now_playing {
                    e.description(generate_line(now_playing));
                } else {
                    e.description("Nothing is playing.");
                }

                e
            })
        }).await?;

    } else {
        ctx.say("Nothing is playing at the moment.").await?;
    }

    Ok(())
}