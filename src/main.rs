extern crate core;

use crate::config::{get_config, Visa};
use crate::visa::VisaData;
use imageproc::image::ImageFormat;
use poise::serenity_prelude::{
  Cache, CacheHttp, CreateAttachment, CreateMessage, EventHandler, FullEvent, GatewayIntents,
  GenericChannelId, GuildId, Http, Member, Mentionable, UserId,
};
use poise::{async_trait, serenity_prelude, Framework, FrameworkOptions};
use std::env;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tracing::{error, info, warn};
use tracing::log::log;
use tracing_subscriber::util::SubscriberInitExt;

mod commands;
mod config;
mod render;
mod visa;

pub struct Data();

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub(crate) static DATA_PATH: OnceLock<String> = OnceLock::new();

pub fn get_data_path() -> &'static String {
  DATA_PATH
    .get()
    .expect("Data path should have been initialised.")
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_file(true)
    .with_line_number(true)
    .without_time()
    .init();

  let mut args = env::args();
  let _ = args.next();

  let working_dir = args.next();
  if let Err(err) = DATA_PATH.set(working_dir.unwrap_or("./".to_string())) {
    error!("Something went horribly wrong: {err}.");
    return;
  }

  let bot_token: String = get_config().bot.token.clone();
  if bot_token.is_empty() {
    warn!("No bot token found. Shutting down.");
    return;
  }

  if let Err(err) = run_bot(bot_token).await {
    error!("Something went wrong while running the bot: {err}.");
    return;
  }
}

async fn run_bot(token: String) -> Result<(), Error> {
  let token = token
    .as_str()
    .parse()
    .map_err(|_| "Invalid token defined for sentinel.")?;

  let framework = Framework::<Data, Error>::builder()
    .options(FrameworkOptions {
      commands: commands::get_commands(),
      ..Default::default()
    })
    .build();
  let intents = GatewayIntents::GUILD_MEMBERS | GatewayIntents::GUILDS;

  let client = serenity_prelude::ClientBuilder::new(token, intents)
    .framework(Box::new(framework))
    .event_handler(Arc::new(RieEventHandler(AtomicBool::new(false))))
    .await;

  client?.start().await?;
  Ok(())
}

pub(crate) async fn handle_member_join(
  http: &Http,
  cache: &Cache,
  guild_id: GuildId,
  new_member_id: UserId,
) -> Result<(), Error> {
  let new_member: Member = http.get_member(guild_id, new_member_id).await?;

  let visa: &Visa = match get_config().find_visa(&guild_id) {
    Some(v) => v,
    None => return Ok(()),
  };

  let channel_id = match visa.channel_id.parse::<u64>() {
    Ok(id) => GenericChannelId::new(id),
    Err(_) => {
      warn!("Invalid channel id: {}", visa.channel_id);
      return Ok(());
    }
  };

  let data = VisaData::create(http, cache, &guild_id, &new_member.user).await?;
  let rendered = render::process_image_for(data)?;

  let mut buffer: Vec<u8> = Vec::with_capacity(1024 * 1024 * 32); // 32 KB
  rendered.write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)?;

  http
    .send_message(
      channel_id,
      vec![CreateAttachment::bytes(buffer, "welcome.png")],
      &CreateMessage::new().content(&new_member.mention().to_string()),
    )
    .await?;

  Ok(())
}

struct RieEventHandler(AtomicBool);

#[async_trait]
impl EventHandler for RieEventHandler {
  async fn dispatch(&self, ctx: &serenity_prelude::Context, event: &FullEvent) {
    match event {
      FullEvent::GuildMemberAddition { new_member, .. } => {
        if let Err(err) = handle_member_join(
          &ctx.http,
          &ctx.cache,
          new_member.guild_id.clone(),
          new_member.user.id.clone(),
        )
        .await
        {
          error!("Failed to handle member join event: {err}.")
        }
      }

      FullEvent::Ready {
        data_about_bot: _, ..
      } => {
        async {
          if self
            .0
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
          {
            match poise::builtins::register_globally(ctx.http(), &commands::get_commands()).await {
              Ok(()) => info!("Successfully registered commands."),
              Err(error) => warn!("Failed to register commands: {error:?}"),
            }
          }
        }
        .await
      }
      _ => {}
    }
  }
}
