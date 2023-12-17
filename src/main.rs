use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;

use anyhow::Context;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use indicatif::ProgressBar;
use serde::Serialize;
use serenity::all::{ChannelId, GatewayIntents, GuildChannel, Message};
use serenity::client::ClientBuilder;
use serenity::futures::stream::{self, StreamExt};
use serenity::model::Timestamp;
use serenity::Client;
use url::Url;

const RUN_IN_PARALLEL: usize = 100;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let guild_id: u64 = 1006923006964154428;
    let token = std::env::var("BOT_TOKEN").unwrap();
    let discord_client = ClientBuilder::new(token, GatewayIntents::MESSAGE_CONTENT).await?;
    let openai_client = async_openai::Client::new();
    let output_filename = "summarizes.jsonl";
    let mut output_file = File::create(output_filename)
        .with_context(|| format!("while opening {output_filename}"))?;

    let help_channel_id = ChannelId::new(1028701743917301892);
    let mut before_timestamp: Option<Timestamp> = None;
    loop {
        let archived_threads = discord_client
            .http
            .get_channel_archived_public_threads(help_channel_id, before_timestamp, Some(100))
            .await?;

        let pb = ProgressBar::new(archived_threads.threads.len() as u64);
        let stream: Vec<anyhow::Result<_>> = pb
            .wrap_stream(
                stream::iter(archived_threads.threads)
                    .map(|channel| {
                        ask_bot_for_summary(&discord_client, &openai_client, guild_id, channel)
                    })
                    .buffered(RUN_IN_PARALLEL),
            )
            .collect()
            .await;

        for result in stream {
            let conv @ SummarizedConversation { archived_timestamp, .. } = result?;
            before_timestamp = archived_timestamp;
            serde_json::to_writer(&mut output_file, &conv)?;
            writeln!(&mut output_file)?;
        }

        if !archived_threads.has_more {
            break;
        }
    }

    output_file.flush()?;

    Ok(())
}

async fn ask_bot_for_summary(
    discord_client: &Client,
    openai_client: &async_openai::Client<OpenAIConfig>,
    guild_id: u64,
    GuildChannel { id, thread_metadata, .. }: GuildChannel,
) -> anyhow::Result<SummarizedConversation> {
    let messages = discord_client.http.get_messages(id, None, None).await?;
    let content = generate_conversation(&messages);
    let conversation_url = discord_conversation_url(guild_id, id);
    Ok(SummarizedConversation {
        id,
        archived_timestamp: thread_metadata.and_then(|tm| tm.archive_timestamp),
        conversation_url,
        body: generate_tech_summary(openai_client, &content).await?,
    })
}

#[derive(Debug, Serialize)]
struct SummarizedConversation {
    id: ChannelId,
    archived_timestamp: Option<Timestamp>,
    conversation_url: Url,
    body: String,
}

fn discord_conversation_url(guild_id: u64, id: ChannelId) -> Url {
    Url::parse(&format!("https://discord.com/channels/{guild_id}/{id}")).unwrap()
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
        .model("gpt-3.5-turbo")
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(TECH_SUMMARIZE_PROMPT)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(format!("```plain text\n{conversation}\n```"))
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    let first_choice = response.choices.into_iter().next().unwrap();
    Ok(first_choice.message.content.unwrap())
}
