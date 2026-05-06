use crate::{get_data_path, Error};
use serde::Deserialize;
use std::fs;
use std::sync::LazyLock;
use poise::serenity_prelude::GuildId;
use tracing::error;

static INSTANCE: LazyLock<Config> = LazyLock::new(|| load_config_safe(get_data_path()));

pub fn get_config() -> &'static LazyLock<Config> {
  &INSTANCE
}

const CONFIG_DEFAULT: &str = include_str!("resources/default-config.toml");

#[derive(Deserialize)]
pub struct Config {
  pub bot: Bot,
  pub visa: Option<Vec<Visa>>,
}

#[derive(Deserialize)]
pub struct Bot {
  pub token: String,
}

#[derive(Deserialize, Clone)]
pub struct Visa {
  #[serde(rename = "guild-id")]
  pub guild_id: String,
  #[serde(rename = "channel-id")]
  pub channel_id: String,
  #[serde(rename = "background-image")]
  pub background_image: String,
  #[serde(rename = "background-color")]
  pub background_color: Option<String>,
}

impl Config {
  pub fn find_visa(&self, guild_id: &GuildId) -> Option<&Visa> {
    self.visa.as_ref().and_then(|v| v.iter().find(|visa| visa.guild_id == guild_id.to_string()))
  }
}

fn load_config_safe(path: &String) -> Config {
  load_config(path).unwrap_or_else(|err| {
    error!("An error occurred trying to load config: {err}.");
    error!("Falling back to default config...");
    toml::from_str(CONFIG_DEFAULT).unwrap()
  })
}

fn load_config(path: &String) -> Result<Config, Error> {
  if !fs::exists(path)? {
    fs::create_dir_all(path)?;
  }

  let config_path = path.clone() + "config.toml";
  if !fs::exists(config_path.clone())? {
    fs::write(config_path.clone(), CONFIG_DEFAULT)?;
  }

  let config = fs::read_to_string(config_path);
  Ok(toml::from_str(config?.as_str())?)
}
