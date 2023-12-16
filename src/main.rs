use serenity::{
    all::{ChannelId, GatewayIntents},
    client::ClientBuilder,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = env!("BOT_TOKEN");
    let client = ClientBuilder::new(token, GatewayIntents::MESSAGE_CONTENT).await?;

    let channel_id = ChannelId::new(1185056786730995913);
    let messages = client.http.get_messages(channel_id, None, None).await?;
    for message in messages {
        dbg!(message);
    }

    Ok(())
}
