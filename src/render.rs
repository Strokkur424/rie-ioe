use crate::visa::VisaData;
use crate::{config, Error};
use ab_glyph::{Font, FontArc, ScaleFont};
use chrono::{DateTime, Datelike, TimeZone, Utc, Weekday};
use config::get_config;
use imageproc::drawing;
use imageproc::drawing::{draw_filled_rect_mut};
use imageproc::image::imageops::{overlay, FilterType};
use imageproc::image::{
  imageops, ColorType, DynamicImage, GenericImage, GenericImageView, ImageReader, Rgba,
};
use imageproc::rect::Rect;
use poise::serenity_prelude::GuildId;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;
use tracing::{info, warn};

static IMAGE_CACHE: OnceLock<Mutex<HashMap<GuildId, DynamicImage>>> = OnceLock::new();

fn resize_image_to(src: DynamicImage, data: &VisaData) -> DynamicImage {
  let maybe_visa = get_config().find_visa(&data.guild_id);
  if maybe_visa.is_none() {
    return src;
  }

  let visa = maybe_visa.unwrap();
  if let Some(res) = &visa.fixed_resolution {
    if res.len() != 2 {
      warn!(
        "Invalid fixed resolution. Provided {}, but expected 2 elements!",
        res.len()
      );
      return src;
    }

    let width: u32 = *res.get(0).unwrap();
    let height: u32 = *res.get(1).unwrap();
    let (src_width, src_height) = GenericImageView::dimensions(&src);

    // Scale
    let scale_x = width as f32 / src_width as f32;
    let scale_y = height as f32 / src_height as f32;

    let scale = f32::max(scale_x, scale_y);
    let resized_width = (src_width as f32 * scale).round() as u32;
    let resized_height = (src_height as f32 * scale).round() as u32;

    // Case 1: src image dimensions are larger.
    //   --> Downsample to fit
    let resized = if src_width > width && src_height > height {
      src.resize_exact(
        resized_width,
        resized_height,
        FilterType::Lanczos3, // high-quality downsampling
      )
    }
    // Case 2: at least one src image dimension is smaller.
    //   --> Upsample fit
    else {
      src.resize_exact(
        resized_width,
        resized_height,
        FilterType::CatmullRom, // decent quality upsampling
      )
    };

    // Crop to dimensions (centered)
    let crop_x = (resized_width.saturating_sub(width)) / 2;
    let crop_y = (resized_height.saturating_sub(height)) / 2;

    let cropped = imageops::crop_imm(&resized, crop_x, crop_y, width, height).to_image();
    return DynamicImage::ImageRgba8(cropped);
  }

  src
}

fn get_background_image(data: &VisaData) -> Result<DynamicImage, Error> {
  if let Some(banner) = &data.user_banner {
    return Ok(pre_process_bg_image(resize_image_to(banner.clone(), data)));
  }

  let mut map = IMAGE_CACHE
    .get_or_init(|| Mutex::new(HashMap::new()))
    .lock()
    .map_err(|_| Error::from("Failed to acquire lock for image cache."))?;

  if let Some(img) = map.get(&data.guild_id) {
    return Ok(img.clone());
  }

  match get_config().find_visa(&data.guild_id) {
    Some(visa) => {
      let img: DynamicImage = pre_process_bg_image(resize_image_to(
        ImageReader::open(visa.background_image.as_str())?.decode()?,
        data,
      ));
      map.insert(data.guild_id.clone(), img.clone());
      Ok(img)
    }
    None => Err(Error::from(format!(
      "No background image set for guild with id {}",
      data.guild_id
    ))),
  }
}

fn compute_text_size(font: &FontArc, scale: f32, text: &str) -> (f32, f32) {
  let scaled_font = font.as_scaled(scale);
  let width = text
    .chars()
    .map(|c| scaled_font.h_advance(font.glyph_id(c)))
    .sum();
  let height = scaled_font.ascent() - scaled_font.descent();

  (width, height)
}

fn pre_process_bg_image(img: DynamicImage) -> DynamicImage {
  let blur_rad: f32 = (img.width() * img.height()) as f32 / 100000.0;
  img.brighten(-30).fast_blur(blur_rad)
}

pub fn process_image_for(data: VisaData) -> Result<DynamicImage, Error> {
  let start_time = SystemTime::now();
  let mut img = get_background_image(&data)?;
  let (width, height) = GenericImageView::dimensions(&img);

  let mut bg_img = DynamicImage::new(width, height, ColorType::Rgba8);
  draw_filled_rect_mut(
    &mut bg_img,
    Rect::at(0, 0).of_size(width, height),
    get_config()
      .find_visa(&data.guild_id)
      .and_then(|v| v.background_color.clone())
      .and_then(|hex| rgba_from_hex(hex.as_str()))
      .unwrap_or_else(|| Rgba([0, 0, 0, 0])),
  );
  overlay(&mut bg_img, &img, 0, 0);

  const FONT_DATA_BOLD: &[u8] = include_bytes!("./fonts/jetbrains-mono/JetBrainsMono-Bold.ttf");
  const FONT_DATA_MEDIUM: &[u8] = include_bytes!("./fonts/jetbrains-mono/JetBrainsMono-Medium.ttf");

  let font_bold = FontArc::try_from_slice(&FONT_DATA_BOLD)?;
  let font_medium = FontArc::try_from_slice(&FONT_DATA_MEDIUM)?;

  let font_size_date = height as f32 * 0.08;

  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    (width as f32 * 0.03) as i32,
    (height as f32 * 0.032) as i32,
    height as f32 * 0.14,
    &font_bold,
    format!("#{}", data.member_count).as_str(),
  );

  let image_size = (height as f32 * 0.7) as u32;
  add_profile_picture(
    &data,
    &mut img,
    (width as f32 * 0.03) as i64,
    (height as f32 * 0.3 * 0.6) as i64,
    image_size,
  );

  let text_anchor_x = (width as f32 * 0.05) as i32 + image_size as i32;
  let text_anchor_y = (height as f32 * 0.25) as i32;

  // Welcome to [Server]!
  let welcome_font_size = height as f32 * 0.07;
  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x,
    text_anchor_y,
    welcome_font_size,
    &font_medium,
    "Welcome to ",
  );
  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x + compute_text_size(&font_medium, welcome_font_size, "Welcome to ").0 as i32,
    text_anchor_y,
    welcome_font_size,
    &font_bold,
    format!("{}!", data.server_name).as_str(),
  );

  // username
  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x,
    (height as f32 * 0.35) as i32,
    height as f32 * 0.1,
    &font_bold,
    data.user_name.as_str(),
  );

  // user id
  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x,
    (height as f32 * 0.45) as i32,
    height as f32 * 0.04,
    &font_medium,
    data.user_id.as_str(),
  );

  // Created on
  let binding = format_time_passed(&data.created_on);
  let time_formatted = binding.as_str();
  let y_anchor = (height as f32 * 0.55) as i32;
  let small_font_scale = height as f32 * 0.04;
  let medium_font_scale = height as f32 * 0.05;

  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x + 10,
    y_anchor + 2,
    small_font_scale,
    &font_medium,
    "Created on",
  );
  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x + (width as f32 * 0.3) as i32,
    y_anchor,
    medium_font_scale,
    &font_medium,
    time_formatted,
  );
  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x,
    y_anchor + (compute_text_size(&font_medium, medium_font_scale, time_formatted).1 * 0.8) as i32,
    font_size_date,
    &font_bold,
    format_created_on(&data.created_on).as_str(),
  );

  // Join
  let binding = format_time(&data.joined_on);
  let time_formatted = binding.as_str();
  let y_anchor = (height as f32 * 0.7) as i32;

  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x + 10,
    y_anchor + 2,
    small_font_scale,
    &font_medium,
    "Joined on",
  );
  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x + (width as f32 * 0.3) as i32,
    y_anchor,
    medium_font_scale,
    &font_medium,
    time_formatted,
  );
  drawing::draw_text_mut(
    &mut img,
    Rgba([0xff, 0xff, 0xff, 0xff]),
    text_anchor_x,
    y_anchor + (compute_text_size(&font_medium, medium_font_scale, time_formatted).1 * 0.8) as i32,
    font_size_date,
    &font_bold,
    format_created_on(&data.joined_on).as_str(),
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
  round_corners(&mut pfp, (scale_to as f64 * 0.13) as u32);
  overlay(img, &pfp, x, y);
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

fn rgba_from_hex(hex: &str) -> Option<Rgba<u8>> {
  let hex = hex.trim_start_matches('#');

  let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
  let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
  let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

  Some(Rgba([r, g, b, 255]))
}
