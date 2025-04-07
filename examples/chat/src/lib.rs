//! Minimal Gossipsub P2P Chat - Rust WASM (browser-only)

// lib.rs
#![cfg(target_arch = "wasm32")]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures::StreamExt;
use js_sys::Date;
use libp2p::{core::Multiaddr, gossipsub, swarm::SwarmEvent};
use libp2p_webrtc_websys as webrtc_websys;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, HtmlElement};

#[wasm_bindgen(start)]
pub fn start() {
    spawn_local(async {
        run().await.expect("run failed");
    });
}

#[wasm_bindgen]
pub fn send_message(msg: String) {
    if let Some(tx) = MSG_TX.lock().unwrap().as_ref() {
        let _ = tx.unbounded_send(msg);
    }
}

use futures::channel::mpsc::{UnboundedSender, UnboundedReceiver, unbounded};
use std::sync::Mutex;
use once_cell::sync::Lazy;

static MSG_TX: Lazy<Mutex<Option<UnboundedSender<String>>>> = Lazy::new(|| Mutex::new(None));

async fn run() -> Result<(), JsError> {
    tracing_wasm::set_as_global_default();
    let body = Body::from_current_window()?;

    // Gossipsub topic
    let topic = gossipsub::IdentTopic::new("xos-demo");

    // Build swarm
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    let config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(5))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(js_error)?;

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_wasm_bindgen()
        .with_other_transport(|key| {
            webrtc_websys::Transport::new(webrtc_websys::Config::new(&key))
        })?
        .with_behaviour(|key| {
            let gs = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                config,
            )?;
            Ok(gs)
        })?
        .build();

    swarm.behaviour_mut().subscribe(&topic)?;

    let (tx, mut rx): (UnboundedSender<String>, UnboundedReceiver<String>) = unbounded();
    *MSG_TX.lock().unwrap() = Some(tx);

    // Broadcast loop
    let mut swarm2 = swarm.clone();
    spawn_local(async move {
        while let Some(msg) = rx.next().await {
            let _ = swarm2.behaviour_mut().publish(topic.clone(), msg.as_bytes());
        }
    });

    // Render loop
    loop {
        match swarm.next().await.unwrap() {
            SwarmEvent::Behaviour(gossipsub::Event::Message { message, .. }) => {
                let text = String::from_utf8_lossy(&message.data);
                body.append_p(&format!("{}", text))?;
            }
            _ => {}
        }
    }
}

/// Convenience DOM
struct Body {
    body: HtmlElement,
    document: Document,
}

impl Body {
    fn from_current_window() -> Result<Self, JsError> {
        let document = web_sys::window().unwrap().document().unwrap();
        let body = document.body().unwrap();
        Ok(Self { body, document })
    }

    fn append_p(&self, msg: &str) -> Result<(), JsError> {
        let val = self.document.create_element("p")?;
        val.set_text_content(Some(msg));
        self.body.append_child(&val)?;
        Ok(())
    }
}

fn js_error<T: ToString>(msg: T) -> JsError {
    JsError::new(&msg.to_string())
}
