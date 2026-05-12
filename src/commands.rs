use crate::{handle_member_join, Context, Data, Error};
use poise::Command;

pub fn get_commands() -> Vec<Command<Data, Error>> {
  vec![force_send_visa()]
}

/// Sends a visa embed to the visa channel in this guild. For testing purposes.
#[poise::command(
  slash_command,
  guild_only,
  ephemeral,
  required_permissions = "ADMINISTRATOR"
)]
async fn force_send_visa(ctx: Context<'_>) -> Result<(), Error> {
  ctx.defer_ephemeral().await?;

  handle_member_join(
    ctx.http(),
    ctx.cache(),
    ctx.guild_id().unwrap(),
    ctx.author_member().await.unwrap().user.id,
  )
  .await?;

  ctx.reply("Success.").await?;
  Ok(())
}
