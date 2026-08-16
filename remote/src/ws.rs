use std::time::Duration;

use actix::Addr;
use actix_web::{HttpRequest, HttpResponse, web};
use actix_ws::{AggregatedMessage, MessageStream, Session};
use futures_util::StreamExt;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use crate::protocol::MoveCmd;
use crate::server::{ClientId, Connect, Disconnect, Frame, Move, Registry, Role, Telemetry};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

pub async fn device_ws(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<Registry>>,
) -> Result<HttpResponse, actix_web::Error> {
    ws_entry(req, stream, srv, Role::Device).await
}

pub async fn ui_ws(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<Registry>>,
) -> Result<HttpResponse, actix_web::Error> {
    ws_entry(req, stream, srv, Role::Ui).await
}

async fn ws_entry(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<Registry>>,
    role: Role,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, session, ws_stream) = actix_ws::handle(&req, stream)?;
    let (tx, rx) = unbounded_channel::<Frame>();
    let Ok(id) = srv.send(Connect { role, tx }).await else {
        return Ok(res);
    };
    actix_web::rt::spawn(session_task(
        session,
        ws_stream,
        rx,
        srv.get_ref().clone(),
        role,
        id,
    ));
    Ok(res)
}

async fn session_task(
    session: Session,
    ws_stream: MessageStream,
    rx: UnboundedReceiver<Frame>,
    srv: Addr<Registry>,
    role: Role,
    id: ClientId,
) {
    actix_web::rt::spawn(forward_to_client(session.clone(), rx));
    read_from_client(session, ws_stream, &srv, role).await;
    let _ = srv.send(Disconnect { role, id }).await;
}

async fn read_from_client(
    mut session: Session,
    ws_stream: MessageStream,
    srv: &Addr<Registry>,
    role: Role,
) {
    // Camera frames (~12-22 KB b64) are sent by esp_websocket_client as 1 KB
    // continuation fragments; plain Message::Text only ever sees the first one.
    let mut ws_stream = ws_stream.aggregate_continuations();
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if session.ping(b"").await.is_err() {
                    return;
                }
            }
            msg = ws_stream.next() => match msg {
                Some(Ok(AggregatedMessage::Text(text))) => handle_text(srv, role, &text).await,
                Some(Ok(AggregatedMessage::Ping(bytes))) => {
                    if session.pong(&bytes).await.is_err() {
                        return;
                    }
                }
                Some(Ok(AggregatedMessage::Close(reason))) => {
                    let _ = session.close(reason).await;
                    return;
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return,
            },
        }
    }
}

async fn forward_to_client(mut session: Session, mut rx: UnboundedReceiver<Frame>) {
    while let Some(frame) = rx.recv().await {
        if session.text(frame.0).await.is_err() {
            break;
        }
    }
}

async fn handle_text(srv: &Addr<Registry>, role: Role, text: &str) {
    match role {
        Role::Ui => {
            if let Ok(cmd) = serde_json::from_str::<MoveCmd>(text) {
                let _ = srv.send(Move { dir: cmd.dir }).await;
            }
        }
        Role::Device => {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(text) {
                let _ = srv.send(Telemetry { data }).await;
            }
        }
    }
}
