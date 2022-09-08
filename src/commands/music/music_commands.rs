use lavalink_rs::model::{TrackQueue};
use log::error;
use regex::Regex;
use serenity::utils::MessageBuilder;
use crate::{Error, Lavalink};
use crate::CowContext;

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en", "List the commands available in the music module.")
)]
pub async fn help(ctx: CowContext<'_>) -> Result<(), Error> {
    ctx.say("`help, join, leave, play, playlist, pause, now_playing, skip, queue`").await?;

    Ok(())
}

pub async fn join_interactive(ctx: &CowContext<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(ctx.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("Join a voice channel first.").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx).await.unwrap().clone();

    let (_, handler) = manager.join_gateway(guild_id, connect_to).await;

    match handler {
        Ok(connection_info) => {
            let lava_client = {
                let data = ctx.discord().data.read().await;
                data.get::<Lavalink>().unwrap().clone()
            };

            lava_client.create_session_with_songbird(&connection_info).await?;
            ctx.say(format!("Joined <#{}>", connect_to)).await?;
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
    description_localized("en", "Join the voice channel you are in.")
)]
pub async fn join(ctx: CowContext<'_>) -> Result<(), Error> {
    join_interactive(&ctx).await
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en", "Make the bot leave the voice channel.")
)]
pub async fn leave(ctx: CowContext<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;
    let serenity = ctx.discord();

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
    description_localized("en", "Play some music.")
)]
pub async fn play(
    ctx: CowContext<'_>,
    #[description = "A YouTube URL or name."] #[rest] query: String)
-> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(channel) => channel.guild_id,
        None => {
            ctx.say("Error finding channel info").await?;
            return Ok(());
        }
    };

    let serenity = ctx.discord();
    let lava_client = {
        let data = serenity.data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    let manager = songbird::get(&serenity).await.unwrap().clone();

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
                ctx.say("Note: This seems to be a playlist. If you want to add all tracks at once, use `playlist` instead of `play`.\n".to_string() + &*message).await?;
                return Ok(())
            }
        }
        ctx.say(message).await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en", "Queue all music from a playlist.")
)]
pub async fn playlist(
    ctx: CowContext<'_>,
    #[description = "A YouTube URL or query to a playlist."] #[rest] query: String)
-> Result<(), Error> {
    if let Some(guild_id) = ctx.guild_id() {
        let serenity = ctx.discord();
        let lava_client = {
            let data = serenity.data.read().await;
            data.get::<Lavalink>().unwrap().clone()
        };

        let manager = songbird::get(&serenity).await.unwrap().clone();

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
                            ctx.say(MessageBuilder::new().push("Added to the queue ").push(tracks.tracks.len()).push(" tracks from ").push_mono_safe(name).push(".")).await?;
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

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en", "Pause the music player.")
)]
pub async fn pause(ctx: CowContext<'_>) -> Result<(), Error> {
    if let Some(guild_id) = ctx.guild_id() {
        let lava_client = {
            let data = ctx.discord().data.read().await;
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
    description_localized("en", "Get the current music playing."),
    aliases("np", "nowplaying")
)]
pub async fn now_playing(ctx: CowContext<'_>) -> Result<(), Error> {
    let lava_client = {
        let data = ctx.discord().data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    if let Some(node) = lava_client.nodes().await.get(ctx.guild_id().0) {
        if let Some(track) = &node.now_playing {
            let info = track.track.info.as_ref().unwrap();
            let re = Regex::new(r#"(?:youtube\.com/(?:[^/]+/.+/|(?:v|e(?:mbed)?)/|.*[?&]v=)|youtu\.be/)([^"&?/\s]{11})"#).unwrap();
            let caps = re.captures(&*info.uri).unwrap();
            let id = caps.get(1).map(|m| m.as_str());
            let server_name = ctx.guild().map(|o| o.name);

            ctx.send(|m| m.embed(|e| {
                 e
                    .author(|a| a.name(match server_name {
                        Some(name) => format!("Now Playing in {}", name),
                        None => "Now Playing".to_string()
                    }))
                    .title(&info.title)
                    .url(&info.uri)
                    .field("Artist", &info.author, true)
                    .field("Duration", format!("{}/{}", crate::util::from_ms(info.position), crate::util::from_ms(info.length)), true);


                if let Some(requester) = track.requester {
                    e.field("Requested By", format!("<@{}>", requester), true);
                }

                if let Some(yt_id) = id {
                    e.thumbnail(format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", yt_id));
                }

                e
            }
            )).await?;
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
    description_localized("en", "Skip the current song.")
)]
pub async fn skip(ctx: CowContext<'_>) -> Result<(), Error> {
    let lava_client = {
        let data = ctx.discord().data.read().await;
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

            page.push_str(&*next_line);
        }

        output.push(page);
    }

    output
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en", "Get the music queue."),
    aliases("q")
)]
pub async fn queue(ctx: CowContext<'_>, page: Option<usize>) -> Result<(), Error> {
    let lava_client = {
        let data = ctx.discord().data.read().await;
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
        let server_name = guild_id.name(&ctx);

        ctx.send(|m| m.embed(|e| {
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
        })).await?;

    } else {
        ctx.say("Nothing is playing at the moment.").await?;
    }

    Ok(())
}