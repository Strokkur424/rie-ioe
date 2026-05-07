use crate::visa::VisaData;
use crate::{config, Error};
use ab_glyph::FontArc;
use chrono::{DateTime, Datelike, TimeZone, Utc, Weekday};
use config::get_config;
use imageproc::image;
use imageproc::image::imageops::FilterType;
use imageproc::image::{DynamicImage, GenericImage, GenericImageView, ImageReader, Rgba};
use poise::serenity_prelude::GuildId;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;
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
  let (width, height) = GenericImageView::dimensions(&img);
  let start_time = SystemTime::now();

  let img = img.brighten(-50);
  let mut img = img.fast_blur(10.0);

  const FONT_DATA_BOLD: &[u8] = include_bytes!("./fonts/jetbrains-mono/JetBrainsMono-Bold.ttf");
  const FONT_DATA_MEDIUM: &[u8] = include_bytes!("./fonts/jetbrains-mono/JetBrainsMono-Medium.ttf");

  let font_bold = FontArc::try_from_slice(&FONT_DATA_BOLD)?;
  let font_medium = FontArc::try_from_slice(&FONT_DATA_MEDIUM)?;

  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.03) as i32,
    (height as f32 * 0.032) as i32,
    height as f32 * 0.14,
    &font_bold,
    format!("#{}", data.member_count).as_str(),
  );

  let image_size = (height as f32 * 0.65) as u32;
  add_profile_picture(
    &data,
    &mut img,
    (width as f32 * 0.03) as i64,
    (height as f32 * 0.4 * 0.5) as i64,
    image_size,
  );

  // Welcome to [Server]!
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.05) as i32 + image_size as i32,
    (height as f32 * 0.25) as i32,
    height as f32 * 0.06,
    &font_medium,
    "Welcome to",
  );
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.22) as i32 + image_size as i32,
    (height as f32 * 0.25) as i32,
    height as f32 * 0.06,
    &font_bold,
    format!("{}!", data.server_name).as_str(),
  );

  // username
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.05) as i32 + image_size as i32,
    (height as f32 * 0.35) as i32,
    height as f32 * 0.1,
    &font_bold,
    data.user_name.as_str(),
  );

  // user id
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.05) as i32 + image_size as i32,
    (height as f32 * 0.45) as i32,
    height as f32 * 0.04,
    &font_medium,
    data.user_id.as_str(),
  );

  // Created on
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.05) as i32 + image_size as i32,
    (height as f32 * 0.56) as i32,
    height as f32 * 0.04,
    &font_medium,
    "Created on",
  );
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.05) as i32 + image_size as i32,
    (height as f32 * 0.60) as i32,
    height as f32 * 0.06,
    &font_bold,
    format_created_on(&data.created_on).as_str(),
  );
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.35) as i32 + image_size as i32,
    (height as f32 * 0.55) as i32,
    height as f32 * 0.045,
    &font_medium,
    format_time_passed(&data.created_on).as_str(),
  );

  // Join
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.05) as i32 + image_size as i32,
    (height as f32 * 0.7) as i32,
    height as f32 * 0.04,
    &font_medium,
    "Joined on",
  );
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.05) as i32 + image_size as i32,
    (height as f32 * 0.74) as i32,
    height as f32 * 0.06,
    &font_bold,
    format_created_on(&data.joined_on).as_str(),
  );
  imageproc::drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.35) as i32 + image_size as i32,
    (height as f32 * 0.69) as i32,
    height as f32 * 0.045,
    &font_medium,
    format_time(&data.joined_on).as_str(),
  );

  info!(
    "Image processing took {}ms",
    SystemTime::now().duration_since(start_time)?.as_millis()
  );
  Ok(img)
}

fn add_profile_picture(data: &VisaData, img: &mut DynamicImage, x: i64, y: i64, scale_to: u32) {
  let pfp = data.user_pfp.clone();
  let mut pfp = pfp.resize(scale_to, scale_to, FilterType::Triangle);
  round_corners(&mut pfp, (scale_to as f64 * 0.1) as u32);
  image::imageops::overlay(img, &pfp, x, y);
}

fn format_time<T: TimeZone>(time: &DateTime<T>) -> String {
  time.to_utc().format("%I:%M %P").to_string()
}

fn format_time_passed(time: &DateTime<Utc>) -> String {
  let dur = Utc::now().signed_duration_since(time);

  if dur.num_days() >= 365 {
    return format!("{} years ago", dur.num_days() / 365);
  }

  if dur.num_days() > 0 {
    return format!("{} days ago", dur.num_days());
  }

  if dur.num_hours() > 0 {
    return format!("{} hours ago", dur.num_hours());
  }

  if dur.num_minutes() > 0 {
    return format!("{} minutes ago", dur.num_minutes());
  }

  format!("{} seconds ago", dur.num_seconds())
}

fn format_created_on<T: TimeZone>(time: &DateTime<T>) -> String {
  let weekday_name = match time.weekday() {
    Weekday::Mon => "Monday",
    Weekday::Tue => "Tuesday",
    Weekday::Wed => "Wednesday",
    Weekday::Thu => "Thursday",
    Weekday::Fri => "Friday",
    Weekday::Sat => "Saturday",
    Weekday::Sun => "Sunday",
  };

  let binding = vec![
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
  ];
  let month_name = binding.get(time.month0() as usize).unwrap();

  let day = time.day();
  let day_string = if day == 1 || (day >= 20 && day % 10 == 1) {
    format!("{}st", day)
  } else if day == 2 || (day >= 20 && day % 10 == 2) {
    format!("{}nd", day)
  } else if day == 3 || (day >= 20 && day % 10 == 3) {
    format!("{}rd", day)
  } else {
    format!("{}th", day)
  };

  format!(
    "{}, {} {} {}",
    weekday_name,
    month_name,
    day_string,
    time.year()
  )
}

fn round_corners(img: &mut DynamicImage, radius: u32) {
  let (w, h) = GenericImageView::dimensions(img);

  for y in 0..h {
    for x in 0..w {
      let dx = if x < radius {
        radius - x
      } else if x >= w - radius {
        x - (w - radius - 1)
      } else {
        0
      };

      let dy = if y < radius {
        radius - y
      } else if y >= h - radius {
        y - (h - radius - 1)
      } else {
        0
      };

      if dx * dx + dy * dy > radius * radius {
        img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
      }
    }
  }
}
