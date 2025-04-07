//! Minimal Gossipsub P2P Chat - Rust WASM (browser-only)

// lib.rs
#![cfg(target_arch = "wasm32")]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use std::rc::Rc;
use std::cell::RefCell;

use futures::StreamExt;
use libp2p::{gossipsub, swarm::SwarmEvent, PeerId};
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
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    tracing_wasm::set_as_global_default();
    let body = Body::from_current_window()?;

    let topic = gossipsub::IdentTopic::new("xos-demo");

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
        .map_err(|e| JsError::new(&format!("config error: {:?}", e)))?;

    let (tx, mut rx): (UnboundedSender<String>, UnboundedReceiver<String>) = unbounded();
    *MSG_TX.lock().unwrap() = Some(tx);

    let swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_wasm_bindgen()
        .with_other_transport(|key| {
            Ok(webrtc_websys::Transport::new(webrtc_websys::Config::new(&key)))
        })?
        .with_behaviour(|key| {
            let gs = gossipsub::Behaviour::<gossipsub::IdentityTransform, gossipsub::AllowAllSubscriptionFilter>::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                config,
            )?;
            Ok(gs)
        })?
        .build();

    let swarm = Rc::new(RefCell::new(swarm));
    swarm.borrow_mut().behaviour_mut().subscribe(&topic)?;

    let tx_swarm = swarm.clone();
    let topic_hash = topic.hash();
    spawn_local(async move {
        while let Some(msg) = rx.next().await {
            let publish_result = {
                let swarm = &mut *tx_swarm.borrow_mut();
                swarm.behaviour_mut().publish(topic.clone(), msg.as_bytes())
            };
    
            match publish_result {
                Ok(_) => {
                    web_sys::console::log_1(&"✅ Message published".into());
                }
                Err(err) => {
                    web_sys::console::error_1(&format!("❌ Publish failed: {:?}", err).into());
                }
            }
        }
    });
    

    let swarm_ref = swarm.clone();
    loop {
        let event = swarm_ref.borrow_mut().next().await;
        if let Some(event) = event {
            match event {
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    web_sys::console::log_1(&format!("✅ Connected to peer: {peer_id}").into());
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    web_sys::console::log_1(&format!("❌ Disconnected from peer: {peer_id}").into());
                }
                SwarmEvent::Behaviour(gossipsub::Event::Message { message, .. }) => {
                    let text = String::from_utf8_lossy(&message.data);
                    body.append_p(&text)?;
                }
                _ => {}
            }
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
        let val = self.document.create_element("p").map_err(|e| JsError::new(&format!("{:?}", e)))?;
        val.set_text_content(Some(msg));
        self.body.append_child(&val).map_err(|e| JsError::new(&format!("{:?}", e)))?;
        Ok(())
    }
}