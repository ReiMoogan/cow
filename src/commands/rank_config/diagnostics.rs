use std::collections::{HashMap, HashSet};
use tracing::error;
use crate::{CowContext, cowdb, Error};
use serenity::{
    model::{
        id::{
            RoleId
        }
    },
    utils::MessageBuilder
};
use crate::{Database, db};

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Scan for discrepancies between server member roles and the stored info."),
    required_permissions = "ADMINISTRATOR",
    guild_cooldown = "900",
    discard_spare_arguments
)]
pub async fn scan(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);
    if let Some(guild_id) = ctx.guild_id() {
        let mut message = MessageBuilder::new();

        let discord_message = ctx.send(|m| {
            m.embeds.clear();
            m.embed(|e| e
                .title("Member Scan")
                .description("Now processing, please wait warmly...")
            )
        }).await?;

        let roles = db.get_roles(guild_id).await?;
        let role_set = roles.into_iter().filter_map(|r| r.role_id).collect::<HashSet<_>>();
        let users = db.get_users(guild_id).await?;
        for u in users {
            if let Ok(member) = guild_id.member(&ctx, u.user).await {
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

        discord_message.edit(ctx, |m| {
            m.embeds.clear();
            m.embed(|e| e
                .title("Member Scan")
                .description(content)
            )
        }).await?;
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Fix any discrepancies between server member roles and the stored info."),
    required_permissions = "ADMINISTRATOR",
    guild_cooldown = "900",
    discard_spare_arguments
)]
pub async fn fix(
    ctx: CowContext<'_>,
    #[description = "Fix users with multiple valid ranks"] option_multiple: Option<bool>,
    #[description = "Remove ranks from people who shouldn't have a rank"] option_remove: Option<bool>,
    #[description = "Demote users who have a higher rank than they should"] option_demote: Option<bool>
) -> Result<(), Error> {
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

        let discord_message = ctx.send(|m| {
            m.embeds.clear();
            m.embed(|e| e
                .title("Role Auto-fix")
                .description("Now fixing roles, please wait warmly...")
            )
        }).await?;
        
        let roles = db.get_roles(guild_id).await?;
        let role_map = roles.into_iter().filter(|r| r.role_id.is_some()).map(|r| (r.role_id.unwrap(), r.min_level)).collect::<HashMap<_, _>>();
        let role_set: HashSet<RoleId> = role_map.keys().cloned().collect(); // Mildly disgusting.
        let users = db.get_users(guild_id).await?;
        for u in users {
            if let Ok(mut member) = guild_id.member(&ctx, u.user).await {
                total += 1;

                let member_role_set: HashSet<RoleId> = HashSet::from_iter(member.roles.iter().cloned());
                let intersection = role_set.intersection(&member_role_set).collect::<HashSet<_>>();
                if let Some(expected_role) = u.role_id {
                    if intersection.contains(&expected_role) && intersection.len() == 1 {
                        continue; // Correct: one role and it's the expected one
                    }
                    total_error += 1;

                    if intersection.is_empty() { // They do not have the role, and need it
                        if let Err(ex) = member.add_role(&ctx, expected_role).await {
                            error!("Failed to add role: {}", ex);
                            count_error += 1;
                        } else {
                            count_trivial += 1;
                        }
                    } else if intersection.len() == 1 { // They have another role in place
                        let existing_role = intersection.into_iter().next().unwrap();
                        let promote = role_map[existing_role] < role_map[&expected_role];
                        if promote || option_demote.unwrap_or(false) { // Promote them
                            if let Err(ex) = member.remove_role(&ctx, existing_role).await {
                                error!("Failed to remove role for demoting: {}", ex);
                                count_error += 1;
                            }

                            if let Err(ex) = member.add_role(&ctx, expected_role).await {
                                error!("Failed to add role for promoting/demoting: {}", ex);
                                count_error += 1;
                            } else if promote {
                                count_trivial += 1;
                            } else {
                                count_demote += 1;
                            }
                        }
                    } else if option_multiple.unwrap_or(false) { // We have multiple to deal with
                        for r in intersection {
                            if *r == expected_role {
                                continue;
                            }

                            if let Err(ex) = member.remove_role(&ctx, r).await {
                                error!("Failed to remove excess roles: {}", ex);
                                count_error += 1;
                            }
                        }

                        if !member.roles.contains(&expected_role) {
                            if let Err(ex) = member.add_role(&ctx, expected_role).await {
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

                    if option_remove.unwrap_or(false) {
                        for r in intersection {
                            if let Err(ex) = member.remove_role(&ctx, r).await {
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

        discord_message.edit(ctx, |m| {
            m.embeds.clear();
            m.embed(|e| e
                .title("Role Auto-fix")
                .description(format!("Processed {total} members in the database with {total_error} errors found:\n\
            - Trivial fixes: {count_trivial}\n\
            - Fixes for multiple roles: {count_multiple}\n\
            - Members with their roles fully revoked: {count_remove}\n\
            - Members demoted: {count_demote}\n\
            - Errors adding/removing roles: {count_error}"))
            )
        }).await?;
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}