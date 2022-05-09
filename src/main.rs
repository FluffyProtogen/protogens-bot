use data::{AppConfiguration, ConfigKey};
use furaffinity::SubmissionData;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};

use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::model::prelude::*;

use std::sync::Arc;
use std::{io, time::*};
use tokio::task;
use tokio::time::sleep;

mod data;
mod db;
mod furaffinity;

#[group]
#[commands(sendhere, stopsending)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let configuration = data::load_configuration().unwrap_or_else(|e| {
        println!("{}", e);
        io::stdin().read_line(&mut String::new()).unwrap();
        panic!();
    });
    let configuration = Arc::new(configuration);

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&configuration.token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    client
        .data
        .write()
        .await
        .insert::<ConfigKey>(Arc::clone(&configuration));

    search_for_protogens(
        Arc::clone(&client.cache_and_http.http),
        Arc::clone(&configuration),
    );

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn sendhere(ctx: &Context, msg: &Message) -> CommandResult {
    if is_admin(&msg.author, msg.guild(ctx).await.unwrap(), ctx).await {
        let data = ctx.data.read().await;
        let login = &data.get::<ConfigKey>().unwrap().postgres_login;

        let client = db::get_connection(&login).await;
        db::start_sending_to_server(
            msg.guild(ctx).await.unwrap().id.0 as i64,
            msg.channel_id.0 as i64,
            &client,
        )
        .await;
        msg.reply(ctx, "Success!").await.unwrap();
    } else {
        msg.reply(ctx, "You must be an administrator to use this command.")
            .await
            .unwrap();
    }

    Ok(())
}

#[command]
async fn stopsending(ctx: &Context, msg: &Message) -> CommandResult {
    if is_admin(&msg.author, msg.guild(ctx).await.unwrap(), ctx).await {
        let data = ctx.data.read().await;
        let login = &data.get::<ConfigKey>().unwrap().postgres_login;

        let client = db::get_connection(login).await;
        db::stop_sending_to_server(msg.guild(ctx).await.unwrap().id.0 as i64, &client).await;
        msg.reply(ctx, "Success!").await.unwrap();
    } else {
        msg.reply(ctx, "You must be an administrator to use this command.")
            .await
            .unwrap();
    }

    Ok(())
}

fn search_for_protogens(http: Arc<Http>, configuration: Arc<data::AppConfiguration>) {
    task::spawn(async move {
        let client = db::get_connection(&configuration.postgres_login).await;

        loop {
            let site_text = furaffinity::get_submissions_site_text(&configuration).await;
            let protogen_ids = furaffinity::parse_proto_ids(&site_text);

            let mut unseen_protogen_ids = vec![];
            for protogen_id in protogen_ids.iter() {
                if !db::has_submission_been_viewed(*protogen_id, &client).await {
                    unseen_protogen_ids.push(protogen_id);
                    db::mark_submission_as_viewed(*protogen_id, &client).await;
                }
            }

            let mut protogen_data = vec![];
            for protogen_id in unseen_protogen_ids.iter() {
                println!("Loading submission {}", protogen_id);
                let site_text =
                    furaffinity::get_submission_info_text(**protogen_id, &configuration).await;
                let data = furaffinity::parse_submission_text(&site_text);

                if !is_blacklisted(&data, &configuration.blacklist) {
                    protogen_data.push(data);
                }
            }

            for channel in db::get_channel_ids(&client).await.iter() {
                let channel = ChannelId(*channel as u64);

                for data in protogen_data.iter() {
                    while let Err(e) = channel.say(&http, &data.url).await {
                        println!("Error sending: {}", e);
                        sleep(Duration::from_millis(3500)).await;
                    }

                    while let Err(e) = channel
                        .send_message(&http, |m| {
                            m.add_embed(|embed| {
                                embed.title(&data.title);
                                embed.description(format!("Tags: {}", &data.tags.join(", ")));
                                embed
                                    .author(|a| a.name(format!("{} - {}", data.artist, data.date)));
                                embed
                            });
                            m
                        })
                        .await
                    {
                        println!("Error sending: {}", e);
                        sleep(Duration::from_millis(3500)).await;
                    }
                }
            }

            sleep(Duration::from_secs(120)).await;
        }
    });
}

fn is_blacklisted(info: &SubmissionData, blacklist: &[String]) -> bool {
    for tag in info.tags.iter() {
        if blacklist.contains(&&tag.to_ascii_lowercase()) {
            return true;
        }
    }

    false
}

async fn is_admin(user: &User, guild: Guild, ctx: &Context) -> bool {
    guild
        .member(ctx, user.id)
        .await
        .unwrap()
        .permissions(ctx)
        .await
        .unwrap()
        .administrator()
}

/*async fn is_admin(user: &User, guild: Guild, ctx: &Context) -> bool {
    let admin_roles = guild
        .roles
        .iter()
        .filter_map(|(roleId, role)| {
            if role.has_permission(Permissions::ADMINISTRATOR) {
                Some(roleId)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for role in admin_roles.iter() {
        if user.has_role(ctx, guild.id, *role).await.unwrap() {
            return true;
        }
    }
    false
}
*/
