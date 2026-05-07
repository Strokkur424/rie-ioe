use crate::Error;
use chrono::{DateTime, Utc};
use imageproc::image::{DynamicImage, ImageReader};
use poise::serenity_prelude::{Cache, GuildId, Http, User};
use std::io::Cursor;

pub struct VisaData {
  pub member_count: u32,
  pub server_name: String,
  pub guild_id: GuildId,
  pub user_name: String,
  pub user_id: String,
  pub user_pfp: DynamicImage,
  pub created_on: DateTime<Utc>,
  pub joined_on: DateTime<Utc>,
}

impl VisaData {
  pub async fn create(http: &Http, cache: &Cache, guild_id: &GuildId, user: &User) -> Result<VisaData, Error> {
    let (name, member_count) = {
      let guild = cache.guild(guild_id.clone()).unwrap();
      let name = guild.name.clone().into_string();
      let member_count = guild.member_count.get();
      (name, member_count)
    };

    Ok(VisaData {
      member_count,
      guild_id: guild_id.clone(),
      server_name: name,
      user_name: user.name.clone().into_string(),
      user_id: user.id.to_string(),
      user_pfp: get_avatar(user).await?,
      created_on: user.id.created_at().to_utc(),
      joined_on: http
        .get_member(guild_id.clone(), user.id)
        .await?
        .joined_at
        .unwrap()
        .to_utc(),
    })
  }
}

async fn get_avatar(user: &User) -> Result<DynamicImage, Error> {
  let url = user.avatar_url().unwrap_or_else(|| {
    format!(
      "https://cdn.discordapp.com/embed/avatars/{}.png",
      (user.id.get() >> 22) % 6
    )
  });

  let bytes = reqwest::get(url).await?.bytes().await?;
  Ok(
    ImageReader::new(Cursor::new(bytes))
      .with_guessed_format()?
      .decode()?,
  )
}
