//! WebSocket broadcast for bucket index delta updates
//!
//! Clients subscribe via `/index/subscribe` and receive binary `BucketDelta`
//! messages after each block (every ~12 seconds on mainnet).
//!
//! ## Protocol
//! 1. Client connects
//! 2. Server sends Hello message (JSON): `{"version":1,"block_number":12345}`
//! 3. Server sends binary BucketDelta after each block
//! 4. If client lags behind, server closes with code 4000 and reason "lagged:<block>"

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use inspire_client::BucketDelta;
use serde::Serialize;
use tokio::sync::broadcast;

/// Broadcast channel capacity (enough for ~10 minutes of blocks)
const BROADCAST_CAPACITY: usize = 64;

/// Bucket index broadcast channel
#[derive(Clone)]
pub struct BucketBroadcast {
    tx: broadcast::Sender<Arc<BucketDelta>>,
}

impl BucketBroadcast {
    /// Create a new broadcast channel
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self { tx }
    }

    /// Broadcast a delta to all connected clients
    ///
    /// Returns the number of receivers that received the message.
    /// Returns 0 if no clients are subscribed.
    pub fn broadcast(&self, delta: BucketDelta) -> usize {
        match self.tx.send(Arc::new(delta)) {
            Ok(count) => {
                tracing::debug!(receivers = count, "Broadcast bucket delta");
                count
            }
            Err(_) => {
                // No receivers - this is fine
                0
            }
        }
    }

    /// Get a receiver for subscribing to deltas
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<BucketDelta>> {
        self.tx.subscribe()
    }

    /// Get the current number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for BucketBroadcast {
    fn default() -> Self {
        Self::new()
    }
}

/// Protocol version for WebSocket subscription
pub const PROTOCOL_VERSION: u16 = 1;

/// Hello message sent on WebSocket connect
#[derive(Debug, Clone, Serialize)]
pub struct WsHello {
    pub version: u16,
    pub block_number: Option<u64>,
}

/// Handle a websocket subscription
///
/// Protocol:
/// 1. Server sends Hello message (JSON) with version and current block
/// 2. Server sends binary BucketDelta messages after each block
/// 3. Server responds to Ping with Pong
/// 4. If client lags, server closes with code 4000 and reason "lagged:<block>"
pub async fn handle_index_subscription(
    socket: WebSocket,
    broadcast: BucketBroadcast,
    current_block: Option<u64>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = broadcast.subscribe();

    // Send Hello message
    let hello = WsHello {
        version: PROTOCOL_VERSION,
        block_number: current_block,
    };
    if let Ok(json) = serde_json::to_string(&hello) {
        if let Err(e) = sender.send(Message::Text(json.into())).await {
            tracing::debug!(error = %e, "Failed to send hello");
            return;
        }
    }

    // Track latest block for lagged close message
    let mut latest_block = current_block;

    // Channel for pong responses from receiver task
    let (pong_tx, mut pong_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
    let (close_tx, mut close_rx) = tokio::sync::oneshot::channel::<()>();

    // Spawn a task to handle incoming messages (ping/pong, close)
    tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(data)) => {
                    tracing::trace!(len = data.len(), "Received ping");
                    let _ = pong_tx.send(data.to_vec()).await;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!(error = %e, "WebSocket receive error");
                    break;
                }
            }
        }
        let _ = close_tx.send(());
    });

    // Send deltas to the client
    loop {
        tokio::select! {
            delta = rx.recv() => {
                match delta {
                    Ok(delta) => {
                        latest_block = Some(delta.block_number);
                        let bytes = delta.to_bytes();
                        if let Err(e) = sender.send(Message::Binary(bytes.into())).await {
                            tracing::debug!(error = %e, "Failed to send delta");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "Client lagged, sending reconnect hint");
                        let reason = match latest_block {
                            Some(block) => format!("lagged:{}", block),
                            None => "lagged".to_string(),
                        };
                        let _ = sender.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                            code: 4000,
                            reason: reason.into(),
                        }))).await;
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            Some(pong_data) = pong_rx.recv() => {
                if let Err(e) = sender.send(Message::Pong(pong_data.into())).await {
                    tracing::debug!(error = %e, "Failed to send pong");
                    break;
                }
            }
            _ = &mut close_rx => {
                break;
            }
        }
    }

    tracing::debug!("WebSocket subscription ended");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_no_receivers() {
        let broadcast = BucketBroadcast::new();
        let delta = BucketDelta {
            block_number: 1,
            updates: vec![(0, 10)],
        };

        // Should not panic with no receivers
        let count = broadcast.broadcast(delta);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_broadcast_with_receiver() {
        let broadcast = BucketBroadcast::new();
        let mut rx = broadcast.subscribe();

        let delta = BucketDelta {
            block_number: 42,
            updates: vec![(100, 5), (200, 10)],
        };

        let count = broadcast.broadcast(delta);
        assert_eq!(count, 1);

        let received = rx.try_recv().unwrap();
        assert_eq!(received.block_number, 42);
        assert_eq!(received.updates.len(), 2);
    }

    #[test]
    fn test_subscriber_count() {
        let broadcast = BucketBroadcast::new();
        assert_eq!(broadcast.subscriber_count(), 0);

        let _rx1 = broadcast.subscribe();
        assert_eq!(broadcast.subscriber_count(), 1);

        let _rx2 = broadcast.subscribe();
        assert_eq!(broadcast.subscriber_count(), 2);

        drop(_rx1);
        // Note: receiver_count may not update immediately after drop
    }
}
