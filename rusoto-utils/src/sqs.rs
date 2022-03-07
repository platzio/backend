use anyhow::{anyhow, Result};
use log::*;
use rusoto_sqs::{DeleteMessageRequest, ListQueuesRequest, ReceiveMessageRequest, Sqs, SqsClient};
use serde::de::DeserializeOwned;
use std::future::Future;

/// Listen for messages from an SQS queue, invoking the given
/// function for every message. If the function returns a successful
/// Result, the message is deleted from the queue.
pub async fn handle_sqs_messages<'a, T, F, Fut>(
    sqs: SqsClient,
    queue_name: &str,
    handler: F,
) -> Result<()>
where
    T: DeserializeOwned,
    F: Fn(T) -> Fut,
    Fut: Future<Output = Result<()>> + Send + 'a,
{
    let queue_url = get_queue_url(&sqs, queue_name).await?;

    loop {
        let res = sqs
            .receive_message(ReceiveMessageRequest {
                queue_url: queue_url.clone(),
                wait_time_seconds: Some(20),
                ..Default::default()
            })
            .await?;
        if let Some(messages) = res.messages {
            for message in messages.into_iter() {
                debug!("Handling message {}", message.message_id.as_ref().unwrap());
                let body = serde_json::from_str::<T>(message.body.as_ref().unwrap())?;
                handler(body).await?;
                sqs.delete_message(DeleteMessageRequest {
                    queue_url: queue_url.clone(),
                    receipt_handle: message.receipt_handle.unwrap(),
                })
                .await?;
            }
        }
    }
}

async fn get_queue_url(sqs: &SqsClient, queue_name: &str) -> Result<String> {
    let res = sqs
        .list_queues(ListQueuesRequest {
            queue_name_prefix: Some(queue_name.to_owned()),
            ..Default::default()
        })
        .await?;

    match res.queue_urls {
        None => Err(anyhow!(format!("SQS queue not found: {}", queue_name))),
        Some(urls) => {
            if urls.len() == 1 {
                Ok(urls.into_iter().next().unwrap())
            } else {
                Err(anyhow!(format!(
                    "Expected exactly one SQS queue URL, got: {:?}",
                    urls
                )))
            }
        }
    }
}
