use std::collections::{HashMap, HashSet};
use log::error;
use crate::CowContext;
use serenity::{
    client::Context,
    model::{
        channel::Message,
        id::{
            RoleId
        }
    },
    framework::standard::{
        CommandResult,
        macros::{
            command
        },
        Args
    },
    utils::MessageBuilder
};
use crate::{Database, db};

#[poise::command(prefix_command, slash_command)]
#[description = "Scan for discrepancies between server member roles and the stored info."]
#[only_in(guilds)]
#[bucket = "diagnostics"]
#[required_permissions("ADMINISTRATOR")]
pub async fn scan(ctx: &CowContext<'_>) -> CommandResult {
    let db = cowdb!(ctx);
    if let Some(guild_id) = ctx.guild_id() {
        let mut message = MessageBuilder::new();

        let mut discord_message = msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| e
            .title("Member Scan")
            .description("Now processing, please wait warmly...")
        )).await?;

        let roles = db.get_roles(guild_id).await?;
        let role_set = roles.into_iter().filter_map(|r| r.role_id).collect::<HashSet<_>>();
        let users = db.get_users(guild_id).await?;
        for u in users {
            if let Ok(member) = guild_id.member(&ctx.http, u.user).await {
                let member_role_set: HashSet<RoleId> = HashSet::from_iter(member.roles.iter().cloned());
                let intersection = role_set.intersection(&member_role_set).collect::<HashSet<_>>();
                if let Some(expected_role) = u.role_id {
                    if intersection.contains(&expected_role) && intersection.len() == 1 {
                        continue; // Correct: one role and it's the expected one
                    }
                    // Either doesn't have the role, wrong role, or too many roles
                    message.push("<@").push(u.user).push("> should have ").role(expected_role);
                    if intersection.is_empty() {
                        message.push(" but doesn't");
                    } else {
                        message.push(" but has: ");
                        intersection.into_iter().for_each(|r| { message.push(" ").role(r).push(" "); });
                    }
                    message.push("\n");
                } else {
                    if intersection.is_empty() {
                        continue; // Correct: no roles
                    }
                    // Has a role, when they shouldn't
                    message.push("<@").push(u.user).push("> has excess roles: ");
                    intersection.into_iter().for_each(|r| { message.push(" ").role(r).push(" "); });
                    message.push("\n");
                }
            }
        }

        let mut content = message.build();
        if content.is_empty() {
            content = "There were no discrepancies between our database and the server members.".to_string();
        }

        discord_message.edit(&ctx.http, |m| m.embed(|e| e
            .title("Member Scan")
            .description(content)
        )).await?;
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command)]
#[description = "Fix any discrepancies between server member roles and the stored info. By default, this will only affect"]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
#[bucket = "diagnostics"]
#[usage = "\"multiple\" to fix users with multiple roles, \"remove\" to remove roles from users, and \"demote\" to modify ranks downwards."]
pub async fn fix(ctx: &CowContext<'_>, mut args: Args) -> CommandResult {
    let db = cowdb!(ctx);
    if let Some(guild_id) = ctx.guild_id() {
        /*
            There are several invalid cases we have to worry about:
            - The user shouldn't have the role, and yet they do have conflicting roles (non-trivial) -> remove
            - The user should have the role, and:
              - they *do not* have any conflicting roles (trivial)
              - they have one conflicting role
                - and they should be higher up (trivial)
                - and they should be lower down (non-trivial) -> demote
              - they have multiple conflicting roles (non-trivial) -> multiple

             The trivial cases will be done by default, and the non-trivial cases can be done by options.
         */

        let (mut count_trivial, mut count_multiple, mut count_remove, mut count_demote, mut count_error, mut total_error, mut total) = (0, 0, 0, 0, 0, 0, 0);

        let (mut option_multiple, mut option_remove, mut option_demote) = (false, false, false);

        while !args.is_empty() {
            let arg = args.single::<String>().unwrap().to_lowercase();
            option_multiple |= arg.contains("multiple");
            option_remove |= arg.contains("remove");
            option_demote |= arg.contains("demote");
        }

        let mut discord_message = msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| e
            .title("Role Auto-fix")
            .description("Now fixing roles, please wait warmly...")
        )).await?;

        let roles = db.get_roles(guild_id).await?;
        let role_map = roles.into_iter().filter(|r| r.role_id.is_some()).map(|r| (r.role_id.unwrap(), r.min_level)).collect::<HashMap<_, _>>();
        let role_set: HashSet<RoleId> = role_map.keys().cloned().collect(); // Mildly disgusting.
        let users = db.get_users(guild_id).await?;
        for u in users {
            if let Ok(mut member) = guild_id.member(&ctx.http, u.user).await {
                total += 1;

                let member_role_set: HashSet<RoleId> = HashSet::from_iter(member.roles.iter().cloned());
                let intersection = role_set.intersection(&member_role_set).collect::<HashSet<_>>();
                if let Some(expected_role) = u.role_id {
                    if intersection.contains(&expected_role) && intersection.len() == 1 {
                        continue; // Correct: one role and it's the expected one
                    }
                    total_error += 1;

                    if intersection.is_empty() { // They do not have the role, and need it
                        if let Err(ex) = member.add_role(&ctx.http, expected_role).await {
                            error!("Failed to add role: {}", ex);
                            count_error += 1;
                        } else {
                            count_trivial += 1;
                        }
                    } else if intersection.len() == 1 { // They have another role in place
                        let existing_role = intersection.into_iter().next().unwrap();
                        let promote = role_map[existing_role] < role_map[&expected_role];
                        if promote || option_demote { // Promote them
                            if let Err(ex) = member.remove_role(&ctx.http, existing_role).await {
                                error!("Failed to remove role for demoting: {}", ex);
                                count_error += 1;
                            }

                            if let Err(ex) = member.add_role(&ctx.http, expected_role).await {
                                error!("Failed to add role for promoting/demoting: {}", ex);
                                count_error += 1;
                            } else if promote {
                                count_trivial += 1;
                            } else {
                                count_demote += 1;
                            }
                        }
                    } else if option_multiple { // We have multiple to deal with
                        for r in intersection {
                            if *r == expected_role {
                                continue;
                            }

                            if let Err(ex) = member.remove_role(&ctx.http, r).await {
                                error!("Failed to remove excess roles: {}", ex);
                                count_error += 1;
                            }
                        }

                        if !member.roles.contains(&expected_role) {
                            if let Err(ex) = member.add_role(&ctx.http, expected_role).await {
                                error!("Failed to add role: {}", ex);
                                count_error += 1;
                            }
                        }

                        count_multiple += 1;
                    }
                } else {
                    if intersection.is_empty() {
                        continue; // Correct: no roles
                    }

                    total_error += 1;

                    if option_remove {
                        for r in intersection {
                            if let Err(ex) = member.remove_role(&ctx.http, r).await {
                                error!("Failed to remove role: {}", ex);
                                count_error += 1;
                            } else {
                                count_remove += 1;
                            }
                        }
                    }
                }
            }
        }

        discord_message.edit(&ctx.http, |m| m.embed(|e| e
            .title("Role Auto-fix")
            .description(format!("Processed {} members in the database with {} errors found:\n\
            - Trivial fixes: {}\n\
            - Fixes for multiple roles: {}\n\
            - Members with their roles fully revoked: {}\n\
            - Members demoted: {}\n\
            - Errors adding/removing roles: {}", total, total_error, count_trivial, count_multiple, count_remove, count_demote, count_error))
        )).await?;
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}