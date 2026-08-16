use std::collections::HashMap;

use actix::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::protocol::{Direction, MoveCmd, UiEvent};

pub type ClientId = usize;

#[derive(Copy, Clone)]
pub enum Role {
    Device,
    Ui,
}

pub struct Frame(pub String);

#[derive(Message)]
#[rtype(result = "ClientId")]
pub struct Connect {
    pub role: Role,
    pub tx: UnboundedSender<Frame>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub role: Role,
    pub id: ClientId,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Move {
    pub dir: Direction,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Telemetry {
    pub data: serde_json::Value,
}

pub struct Registry {
    next_id: ClientId,
    devices: HashMap<ClientId, UnboundedSender<Frame>>,
    uis: HashMap<ClientId, UnboundedSender<Frame>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            devices: HashMap::new(),
            uis: HashMap::new(),
        }
    }

    fn push_uis(&self, evt: &UiEvent) {
        let Ok(json) = serde_json::to_string(evt) else {
            return;
        };
        for tx in self.uis.values() {
            let _ = tx.send(Frame(json.clone()));
        }
    }

    fn push_devices(&self, dir: Direction) {
        let Ok(json) = serde_json::to_string(&MoveCmd { dir }) else {
            return;
        };
        for tx in self.devices.values() {
            let _ = tx.send(Frame(json.clone()));
        }
    }

    fn device_online(&self) -> UiEvent {
        UiEvent::Device {
            online: !self.devices.is_empty(),
        }
    }
}

impl Actor for Registry {
    type Context = Context<Self>;
}

impl Handler<Connect> for Registry {
    type Result = ClientId;

    fn handle(&mut self, msg: Connect, _ctx: &mut Self::Context) -> Self::Result {
        let id = self.next_id;
        self.next_id += 1;
        match msg.role {
            Role::Device => {
                self.devices.insert(id, msg.tx);
                self.push_uis(&self.device_online());
            }
            Role::Ui => {
                let hello = serde_json::to_string(&self.device_online()).unwrap_or_default();
                let _ = msg.tx.send(Frame(hello));
                self.uis.insert(id, msg.tx);
            }
        }
        id
    }
}

impl Handler<Disconnect> for Registry {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _ctx: &mut Self::Context) {
        match msg.role {
            Role::Device => {
                if self.devices.remove(&msg.id).is_some() {
                    self.push_uis(&self.device_online());
                }
            }
            Role::Ui => {
                self.uis.remove(&msg.id);
            }
        }
    }
}

impl Handler<Move> for Registry {
    type Result = ();

    fn handle(&mut self, msg: Move, _ctx: &mut Self::Context) {
        self.push_devices(msg.dir);
    }
}

impl Handler<Telemetry> for Registry {
    type Result = ();

    fn handle(&mut self, msg: Telemetry, _ctx: &mut Self::Context) {
        self.push_uis(&UiEvent::Telemetry { data: msg.data });
    }
}
