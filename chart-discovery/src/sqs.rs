use anyhow::{anyhow, Result};
use aws_types::region::Region;
use log::*;
use serde::de::DeserializeOwned;
use std::future::Future;

/// Listen for messages from an SQS queue, invoking the given
/// function for every message. If the function returns a successful
/// Result, the message is deleted from the queue.
pub async fn handle_messages<'a, T, F, Fut>(
    queue_region: Region,
    queue_name: &str,
    handler: F,
) -> Result<()>
where
    T: DeserializeOwned,
    F: Fn(T) -> Fut,
    Fut: Future<Output = Result<()>> + Send + 'a,
{
    let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let sqs_config = aws_sdk_sqs::config::Builder::from(&shared_config)
        .region(queue_region)
        .build();
    let sqs = aws_sdk_sqs::Client::from_conf(sqs_config);
    let queue_url = get_queue_url(&sqs, queue_name).await?;

    loop {
        let res = sqs
            .receive_message()
            .queue_url(queue_url.clone())
            .wait_time_seconds(20)
            .send()
            .await?;
        if let Some(messages) = res.messages {
            for message in messages.into_iter() {
                debug!("Handling message {}", message.message_id.as_ref().unwrap());
                let body = serde_json::from_str::<T>(message.body.as_ref().unwrap())?;
                handler(body).await?;
                sqs.delete_message()
                    .queue_url(queue_url.clone())
                    .receipt_handle(message.receipt_handle.unwrap())
                    .send()
                    .await?;
            }
        }
    }
}

async fn get_queue_url(sqs: &aws_sdk_sqs::Client, queue_name: &str) -> Result<String> {
    let res = sqs
        .list_queues()
        .queue_name_prefix(queue_name)
        .send()
        .await?;

    match res.queue_urls {
        None => Err(anyhow!("SQS queue not found: {}", queue_name)),
        Some(urls) => {
            if urls.len() == 1 {
                Ok(urls.into_iter().next().unwrap())
            } else {
                Err(anyhow!(
                    "Expected exactly one SQS queue URL, got: {:?}",
                    urls
                ))
            }
        }
    }
}
