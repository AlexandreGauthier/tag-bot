mod logging;
use logging::*;

use serde::Deserialize;

use serenity::http::Http;
use serenity::model::user::User;
use serenity::model::*;
use serenity::model::{
    channel::{Message, Reaction, ReactionType},
    gateway::Ready,
};
use serenity::prelude::*;

use std::fs::File;
use std::io::prelude::*;

type ChannelMap = std::collections::HashMap<id::ChannelId, Vec<TaggedMessage>>;

const CONFIG_FILE_LOCATION: &str = "bot-config.toml";

#[derive(Deserialize, Clone)]
struct Tag {
    channel_target: id::ChannelId,
    emoji_name: String,
    message_counter: u16,
}

#[derive(Deserialize)]
struct Config {
    token: String,
    roles: RolePermissions,
    tags: Vec<Tag>,
}

#[derive(Deserialize)]
struct RolePermissions {
    console: u64,
    tag: u64,
}

struct TaggedMessage {
    message_id: id::MessageId,
    counter: u16,
}

/// For use in serenity's Context::data to save state across handler calls.
struct ConfigType;
impl TypeMapKey for ConfigType {
    type Value = Config;
}

/// For use in serenity's Context::data to save state across handler calls.
struct ChannelMapType;
impl TypeMapKey for ChannelMapType {
    type Value = ChannelMap;
}

struct Handler;
impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!tag help" {
            const HELP_MSG: &str = "How to tag a message: \n \n 1. React with the appropriate emoji. \n 2. Wait for me to move it \n 3. ??? \n 4. Profit!";
            match msg.channel_id.say(&ctx.http, HELP_MSG) {
                Ok(_) => (),
                Err(e) => BotError::Discord(e).log(),
            }
        };

        let mut data = ctx.data.write();
        let channel_map = data.get_mut::<ChannelMapType>().unwrap();
        let tagged_messages = match channel_map.get_mut(&msg.channel_id) {
            Some(vec) => vec,
            None => {
                channel_map.insert(msg.channel_id, Vec::<TaggedMessage>::new());
                channel_map.get_mut(&msg.channel_id).unwrap()
            }
        };

        for message in tagged_messages {
            if message.counter == 0 {
                match ctx
                    .http
                    .delete_message(u64::from(msg.channel_id), u64::from(message.message_id))
                {
                    Err(e) => BotError::Discord(e).log(),
                    Ok(_) => {}
                }
            } else {
                message.counter -= 1;
            }
        }
    }

    fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        match &reaction.emoji {
            ReactionType::Custom {
                animated: _,
                id: _,
                name,
            } => {
                let mut data = ctx.data.write();
                let tags = data.get_mut::<ConfigType>().unwrap().tags.to_owned();
                let role = data.get_mut::<ConfigType>().unwrap().roles.tag.to_owned();

                let message = reaction.message(&ctx.http).unwrap_gracefully();
                let channel_map = data.get_mut::<ChannelMapType>().unwrap();
                let tagged_messages = match channel_map.get_mut(&reaction.channel_id) {
                    Some(vec) => vec,
                    None => {
                        channel_map.insert(reaction.channel_id, Vec::<TaggedMessage>::new());
                        channel_map.get_mut(&reaction.channel_id).unwrap()
                    }
                };
                let tagging_user = &reaction.user(&ctx.http).unwrap_gracefully();
                match tagging_user.has_role(&ctx.http, reaction.guild_id.unwrap(), role) {
                    Ok(has_perms) => {
                        format!("{} {}", has_perms, role).log();
                        if has_perms {
                            tag_message(
                                &ctx.http,
                                &message,
                                &tags,
                                tagged_messages,
                                name.as_ref().unwrap(),
                                tagging_user,
                            )
                            .unwrap_gracefully();
                        }
                    }
                    Err(e) => BotError::Discord(e).log(),
                }
            }
            ReactionType::Unicode(_) => {}
            ReactionType::__Nonexhaustive => unreachable!(),
        }
    }

    fn ready(&self, _: Context, ready: Ready) {
        format!("Connected to server as {}", ready.user.name).log();
    }
}

/// Checks if reaction has associated tag. If so, cite it in the correct location and tag the original message.
fn tag_message(
    http: &Http,
    message: &Message,
    tags: &Vec<Tag>,
    tagged_messages: &mut Vec<TaggedMessage>,
    name: &String,
    tagging_user: &User,
) -> Result<()> {
    for tag in tags {
        if name == &tag.emoji_name {
            let original_user = &message.author;

            if true {
                // Log action
                format!("User {} tagged post {}", tagging_user.tag(), message.id).log();

                // React with a checkmark
                message.react(http, ReactionType::Unicode("✅".to_string()))?;

                // Log action
                format!("User {} tagged post {}", tagging_user.tag(), message.id).log();

                // Cite original message in target channel
                let mut new_message = format!(
                    "{} says (tagged by {})\n> {}",
                    original_user.mention(),
                    tagging_user.mention(),
                    message.content
                );
                for attachment in &message.attachments {
                    new_message.push_str("\n");
                    new_message.push_str(&attachment.url)
                }

                tag.channel_target.say(http, new_message)?;

                // Add original message in tagged list
                let entry = TaggedMessage {
                    message_id: message.id,
                    counter: tag.message_counter,
                };

                tagged_messages.push(entry);
            }
        }
    }

    Ok(())
}

/// Opens config toml file and parses it into a Config struct.
fn read_parse_config() -> Result<Config> {
    let mut config_file = String::new();
    File::open(CONFIG_FILE_LOCATION)?.read_to_string(&mut config_file)?;
    Ok(toml::from_str(&config_file)?)
}

fn main() {
    format!("Starting tag-bot.").log();
    format!("Loading configuration file: {}.", CONFIG_FILE_LOCATION).log();
    let config = read_parse_config().unwrap_gracefully();

    let mut client = match Client::new(&config.token, Handler) {
        Ok(c) => c,
        Err(e) => {
            BotError::Discord(e).log();
            panic!();
        }
    };

    format!("Connecting to Discord API.").log();
    client.data.write().insert::<ChannelMapType>(ChannelMap::new());
    client.data.write().insert::<ConfigType>(config);

    if let Err(e) = client.start() {
        BotError::Discord(e).log();
    }
}
