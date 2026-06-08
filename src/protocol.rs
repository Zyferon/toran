//! Wire protocol for the Python SDK ⇄ Rust core Unix socket.
//!
//! We use a simple length-prefixed JSON protocol instead of FlatBuffers
//! to keep the build toolchain minimal (FlatBuffers requires a schema
//! compiler). The frame is:
//!
//!   [4 bytes big-endian length N] [N bytes UTF-8 JSON]
//!
//! JSON keeps the wire format debuggable. We can swap in FlatBuffers
//! later without changing the rest of the system.

use crate::policy::evaluator::{Decision, Request};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Evaluate {
        request: Request,
        agent_id: String,
        session_id: String,
    },
    Wait {
        approval_id: String,
        timeout_secs: u64,
        token: String,
    },
    Ping,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Decision {
        decision: Decision,
        approval_id: Option<String>,
        notify_token: Option<String>,
    },
    Approved,
    Denied,
    Timeout,
    Error {
        message: String,
    },
    Pong,
    Bye,
}

const MAX_FRAME: usize = 16 * 1024 * 1024; // 16 MiB safety cap

pub async fn write_frame<W: AsyncWriteExt + Unpin>(w: &mut W, msg: &ServerMessage) -> Result<()> {
    let bytes = serde_json::to_vec(msg)?;
    if bytes.len() > MAX_FRAME {
        return Err(anyhow!("frame too large: {} bytes", bytes.len()));
    }
    let len = (bytes.len() as u32).to_be_bytes();
    w.write_all(&len).await?;
    w.write_all(&bytes).await?;
    w.flush().await?;
    Ok(())
}

pub async fn read_frame<R: AsyncReadExt + Unpin>(r: &mut R) -> Result<ClientMessage> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME {
        return Err(anyhow!("frame too large: {} bytes", len));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    let msg: ClientMessage = serde_json::from_slice(&buf)?;
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip() {
        let req = Request {
            function_name: "send_email".into(),
            args: Default::default(),
            context: Default::default(),
        };
        let original = ClientMessage::Evaluate {
            request: req,
            agent_id: "agent-1".into(),
            session_id: "sess-1".into(),
        };
        let mut buf: Vec<u8> = Vec::new();
        let bytes = serde_json::to_vec(&original).unwrap();
        let len = (bytes.len() as u32).to_be_bytes();
        buf.extend_from_slice(&len);
        buf.extend_from_slice(&bytes);

        let mut slice: &[u8] = &buf;
        let parsed = read_frame(&mut slice).await.unwrap();
        match parsed {
            ClientMessage::Evaluate { request, .. } => {
                assert_eq!(request.function_name, "send_email");
            }
            _ => panic!("wrong variant"),
        }
    }
}
