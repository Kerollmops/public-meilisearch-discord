use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
};
use serenity::{
    all::{ChannelId, GatewayIntents, GuildChannel, Message},
    client::ClientBuilder,
};
use std::fmt::Write as _;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let guild_id: u64 = 1006923006964154428;
    let token = std::env::var("BOT_TOKEN").unwrap();
    let client = ClientBuilder::new(token, GatewayIntents::MESSAGE_CONTENT).await?;

    let help_channel_id = ChannelId::new(1028701743917301892);
    let archived_threads = client
        .http
        .get_channel_archived_public_threads(help_channel_id, None, None)
        .await?;

    let openai_client = async_openai::Client::new();

    // let channel_id = ChannelId::new(1183817817846456350);
    for GuildChannel { id, .. } in archived_threads.threads.into_iter().take(usize::MAX) {
        let messages = client.http.get_messages(id, None, None).await?;
        let content = generate_conversation(&messages);
        eprintln!("https://discord.com/channels/{guild_id}/{id}");
        eprintln!("{content}");
        let summary = generate_tech_summary(&openai_client, &content).await?;
        eprintln!("-------------- SUMMARY --------------");
        eprintln!("{summary}");
        eprintln!("-------------------------------------");
    }

    Ok(())
}

fn generate_conversation(messages: &[Message]) -> String {
    let mut output = String::new();
    for message in messages.iter().rev() {
        let _ = writeln!(&mut output, "<@{}>: {}", message.author.id, message.content);
        let _ = writeln!(&mut output);
    }
    output
}

// TODO use the system role to setup the assistant
//      https://platform.openai.com/docs/guides/text-generation/chat-completions-api
async fn generate_tech_summary(
    client: &async_openai::Client<OpenAIConfig>,
    conversation: &str,
) -> anyhow::Result<String> {
    const TECH_SUMMARIZE_PROMPT: &str = include_str!("../tech-summarize.prompt.txt");

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4")
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(format!(
                "```plain text
                {conversation}
                ```

                {TECH_SUMMARIZE_PROMPT}"
            ))
            .build()?
            .into()])
        .build()?;

    let response = client.chat().create(request).await?;
    let first_choice = response.choices.into_iter().next().unwrap();
    Ok(first_choice.message.content.unwrap())
}
