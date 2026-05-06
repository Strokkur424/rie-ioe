use crate::visa::VisaData;
use crate::{config, Error};
use config::get_config;
use imageproc::image::{DynamicImage, ImageReader};
use poise::serenity_prelude::GuildId;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;
use imageproc::image::imageops::brighten;
use tracing::info;

static IMAGE_CACHE: OnceLock<Mutex<HashMap<GuildId, DynamicImage>>> = OnceLock::new();

fn load_background_image(guild_id: &GuildId) -> Result<DynamicImage, Error> {
  let mut map = IMAGE_CACHE
    .get_or_init(|| Mutex::new(HashMap::new()))
    .lock()
    .map_err(|_| Error::from("Failed to acquire lock for image cache."))?;

  if let Some(img) = map.get(&guild_id) {
    return Ok(img.clone());
  }

  match get_config().find_visa(&guild_id) {
    Some(visa) => {
      let img: DynamicImage = ImageReader::open(visa.background_image.as_str())?.decode()?;
      map.insert(guild_id.clone(), img.clone());
      Ok(img)
    }
    None => Err(Error::from(format!(
      "No background image set for guild with id {}",
      guild_id
    ))),
  }
}

pub fn process_image_for(data: VisaData) -> Result<DynamicImage, Error> {
  let img = load_background_image(&data.guild_id)?;
  let start_time = SystemTime::now();

  let img = img.brighten(-30);
  let img = img.fast_blur(10.0);

  info!("Image processing took {}ms", (SystemTime::now().duration_since(start_time)?.as_millis()));
  Ok(img)
}
