//! 再接続付きWebSocketクライアント(ソース共通)

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::{interval, sleep, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::model::Eew;

pub type Stream = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type Parse = fn(&str) -> Result<Option<Eew>>;
pub type OnStatus = Arc<dyn Fn(Status) + Send + Sync>;

const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// 接続状態(UI表示用)
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", content = "detail", rename_all = "snake_case")]
pub enum Status {
    Connecting,
    Connected,
    Disconnected(String),
}

/// 生存確認の方法
#[derive(Debug, Clone, Copy)]
pub enum Keepalive {
    /// テキストメッセージを送る
    Text(Duration, &'static str),
    /// WebSocketのPingフレームを送る(Pongで生存確認)
    Frame(Duration),
}

pub struct Options {
    pub name: &'static str,
    pub keepalive: Keepalive,
    /// この時間なにも受信しなければ切断とみなす。None なら判定しない。
    pub idle_timeout: Option<Duration>,
    pub parse: Parse,
}

pub async fn plain(url: &'static str) -> Result<Stream> {
    Ok(connect_async(url).await?.0)
}

/// 接続→受信→切断を繰り返す。再接続は指数バックオフ(安定して接続できたらリセット)。
pub async fn run<C, F>(
    opts: Options,
    connect: C,
    tx: UnboundedSender<Eew>,
    on_status: OnStatus,
) where
    C: Fn() -> F,
    F: Future<Output = Result<Stream>>,
{
    let mut backoff = Duration::from_secs(1);
    loop {
        let started = Instant::now();
        on_status(Status::Connecting);
        let result = match connect().await {
            Ok(ws) => {
                eprintln!("[{}] connected", opts.name);
                on_status(Status::Connected);
                session(&opts, ws, &tx).await
            }
            Err(e) => Err(e),
        };
        let reason = match result {
            Ok(()) => "closed".to_string(),
            Err(e) => format!("{e:#}"),
        };
        eprintln!("[{}] disconnected: {reason}", opts.name);
        on_status(Status::Disconnected(reason));
        if tx.is_closed() {
            return;
        }
        if started.elapsed() > Duration::from_secs(60) {
            backoff = Duration::from_secs(1);
        }
        sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

async fn session(opts: &Options, ws: Stream, tx: &UnboundedSender<Eew>) -> Result<()> {
    let (mut write, mut read) = ws.split();
    let (Keepalive::Text(every, _) | Keepalive::Frame(every)) = opts.keepalive;
    let mut ticker = interval(every);
    ticker.tick().await;
    loop {
        tokio::select! {
            _ = ticker.tick() => match opts.keepalive {
                Keepalive::Text(_, msg) => write.send(Message::text(msg)).await?,
                Keepalive::Frame(_) => write.send(Message::Ping(Default::default())).await?,
            },
            msg = recv(&mut read, opts.idle_timeout) => {
                let Some(msg) = msg? else { return Ok(()) };
                match msg {
                    Message::Text(t) => match (opts.parse)(&t) {
                        Ok(Some(eew)) => {
                            if tx.send(eew).is_err() {
                                return Ok(());
                            }
                        }
                        Ok(None) => {}
                        Err(e) => eprintln!("[{}] unparsable message: {e:#}", opts.name),
                    },
                    Message::Close(_) => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

async fn recv<S>(read: &mut S, idle: Option<Duration>) -> Result<Option<Message>>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let next = match idle {
        Some(d) => match timeout(d, read.next()).await {
            Ok(n) => n,
            Err(_) => bail!("idle timeout"),
        },
        None => read.next().await,
    };
    Ok(next.transpose()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 実サーバへ接続して最初のメッセージ(heartbeat)を受け取れること。
    /// `cargo test -- --ignored` で実行(ネットワーク使用)。
    #[tokio::test]
    #[ignore]
    async fn connects_to_wolfx_and_receives_heartbeat() {
        let ws = plain(crate::sources::wolfx::URL).await.unwrap();
        let (_, mut read) = ws.split();
        let first = timeout(Duration::from_secs(20), read.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(first.into_text().unwrap().contains("heartbeat"));
    }

    /// P2P地震情報はPingフレームにPongで応答すること(生存確認に使うため)
    #[tokio::test]
    #[ignore]
    async fn p2pquake_answers_ping_frames() {
        let ws = plain(crate::sources::p2p_ws::URL).await.unwrap();
        let (mut write, mut read) = ws.split();
        write.send(Message::Ping(Default::default())).await.unwrap();
        let pong = timeout(Duration::from_secs(15), async {
            while let Some(m) = read.next().await {
                if let Message::Pong(_) = m.unwrap() {
                    return true;
                }
            }
            false
        })
        .await
        .unwrap();
        assert!(pong);
    }
}
