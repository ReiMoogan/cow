use lavalink_rs::model::{TrackQueue};
use log::error;
use regex::Regex;
use serenity::client::Context;
use serenity::framework::standard::{CommandResult, Args};
use serenity::model::channel::{Message};
use serenity::framework::standard::macros::{command};
use serenity::utils::MessageBuilder;
use crate::Lavalink;

#[command]
#[aliases(p)]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "`help, join, leave, play, playlist, pause, now_playing, skip, queue`").await?;

    Ok(())
}

async fn join_interactive(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.channel_id.say(&ctx.http, "Join a voice channel first.").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx).await.unwrap().clone();

    let (_, handler) = manager.join_gateway(guild_id, connect_to).await;

    match handler {
        Ok(connection_info) => {
            let lava_client = {
                let data = ctx.data.read().await;
                data.get::<Lavalink>().unwrap().clone()
            };

            lava_client.create_session_with_songbird(&connection_info).await?;
            msg.channel_id.say(&ctx.http, format!("Joined <#{}>", connect_to)).await?;
        }
        Err(ex) => {
            msg.channel_id.say(&ctx.http, "Failed to join your VC...").await?;
            error!("Error joining the channel: {}", ex)
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    join_interactive(ctx, msg).await
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await.unwrap().clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(ex) = manager.remove(guild_id).await {
            error!("Failed to disconnect: {}", ex);
        }

        {
            // Free up the LavaLink client.
            let data = ctx.data.read().await;
            let lava_client = data.get::<Lavalink>().unwrap().clone();
            lava_client.destroy(guild_id.0).await?;
        }

        msg.channel_id.say(&ctx.http, "Disconnected from VC. Goodbye!").await?;
    } else {
        msg.channel_id.say(&ctx.http, "I'm not in a VC.").await?;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    if args.is_empty() {
        msg.channel_id.say(&ctx.http, "Please enter a query or link.").await?;
    }

    let query = args.message().to_string();

    let guild_id = match ctx.cache.guild_channel(msg.channel_id) {
        Some(channel) => channel.guild_id,
        None => {
            
            msg.channel_id.say(&ctx.http, "Error finding channel info").await?;

            return Ok(());
        }
    };

    let lava_client = {
        let data = ctx.data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    let manager = songbird::get(ctx).await.unwrap().clone();

    if manager.get(guild_id).is_none() {
        if let Err(ex) = join_interactive(ctx, msg).await {
            msg.channel_id.say(&ctx.http, "Failed to connect to voice channel; maybe I don't have permissions?").await?;
            error!("Failed to connect to vc: {}", ex);
            return Ok(());
        }
    }

    if let Some(_handler) = manager.get(guild_id) {
        let query_information = lava_client.auto_search_tracks(&query).await?;

        if query_information.tracks.is_empty() {
            msg.channel_id.say(&ctx, "Could not find any video of the search query.").await?;
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
                msg.channel_id.say(&ctx.http, "Note: This seems to be a playlist. If you want to add all tracks at once, use `playlist` instead of `play`.\n".to_string() + &*message).await?;
                return Ok(())
            }
        }
        msg.channel_id.say(&ctx.http, message).await?;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn playlist(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    if let Some(guild_id) = msg.guild_id {
        if args.is_empty() {
            msg.channel_id.say(&ctx.http, "Please enter a query or link.").await?;
            return Ok(())
        }

        let query = args.message().to_string();



        let lava_client = {
            let data = ctx.data.read().await;
            data.get::<Lavalink>().unwrap().clone()
        };

        let manager = songbird::get(ctx).await.unwrap().clone();

        if manager.get(guild_id).is_none() {
            if let Err(ex) = join_interactive(ctx, msg).await {
                msg.channel_id.say(&ctx.http, "Failed to connect to voice channel; maybe I don't have permissions?").await?;
                error!("Failed to connect to vc: {}", ex);
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
                            msg.channel_id.say(&ctx.http, MessageBuilder::new().push("Added to the queue ").push(tracks.tracks.len()).push(" tracks from ").push_mono_safe(name).push(".")).await?;
                        } else {
                            msg.channel_id.say(&ctx.http, format!("Added to the queue {} tracks.", tracks.tracks.len())).await?;
                        }
                    } else {
                        msg.channel_id.say(&ctx.http, format!("Added to the queue {} tracks.", tracks.tracks.len())).await?;
                    }
                }
                Err(ex) => {
                    error!("Failed to load tracks: {}", ex);
                    msg.channel_id.say(&ctx, "Could not load any tracks from the given input.").await?;
                }
            }
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    if let Some(guild_id) = msg.guild_id {
        let lava_client = {
            let data = ctx.data.read().await;
            data.get::<Lavalink>().unwrap().clone()
        };

        if let Some(node) = lava_client.nodes().await.get(&guild_id.0) {
            if node.is_paused {
                if let Err(ex) = lava_client.set_pause(guild_id.0, false).await {
                    error!("Failed to unpause music: {}", ex);
                } else {
                    msg.channel_id.say(&ctx.http, "Unpaused the player.").await?;
                }
            } else if let Err(ex) = lava_client.pause(guild_id.0).await {
                error!("Failed to pause music: {}", ex);
            } else {
                msg.channel_id.say(&ctx.http, "Paused the player.").await?;
            }
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[aliases(np, nowplaying)]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    let lava_client = {
        let data = ctx.data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    let guild_id = msg.guild_id.unwrap();

    if let Some(node) = lava_client.nodes().await.get(&guild_id.0) {
        if let Some(track) = &node.now_playing {
            let info = track.track.info.as_ref().unwrap();
            let re = Regex::new(r#"(?:youtube\.com/(?:[^/]+/.+/|(?:v|e(?:mbed)?)/|.*[?&]v=)|youtu\.be/)([^"&?/\s]{11})"#).unwrap();
            let caps = re.captures(&*info.uri).unwrap();
            let id = caps.get(1).map(|m| m.as_str());
            let server_name = guild_id.name(&ctx);

            msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
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
            msg.channel_id.say(&ctx.http, "Nothing is playing at the moment.").await?;
        }
    } else {
        msg.channel_id.say(&ctx.http, "Nothing is playing at the moment.").await?;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let lava_client = {
        let data = ctx.data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    if let Some(track) = lava_client.skip(msg.guild_id.unwrap()).await {
        msg.channel_id.say(&ctx.http, MessageBuilder::new().push("Skipped: ").push_mono_line_safe(&track.track.info.as_ref().unwrap().title)).await?;

        // Need to check if it's empty, so we can stop playing (can crash if we don't check)
        if let Some(node) = lava_client.nodes().await.get(&msg.guild_id.unwrap().0) {
            if node.now_playing.is_none() {
                if let Err(ex) = lava_client.stop(msg.guild_id.unwrap()).await {
                    error!("Failed to stop music: {}", ex);
                }
            }
        }
    } else {
        msg.channel_id.say(&ctx.http, "There is nothing to skip.").await?;
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

#[command]
#[only_in(guilds)]
#[aliases(q)]
async fn queue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let lava_client = {
        let data = ctx.data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    let mut page_num = if let Ok(arg_page) = args.single::<usize>() {
        arg_page
    } else {
        1
    };

    let guild_id = msg.guild_id.unwrap();
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

        msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
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
        msg.channel_id.say(&ctx.http, "Nothing is playing at the moment.").await?;
    }

    Ok(())
}