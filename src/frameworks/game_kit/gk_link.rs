/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Local (not upstream): `GKPeerPickerController` and `GKSession` backed by a
//! TCP link between two touchHLE instances, so that games using GameKit
//! "Bluetooth" peer-to-peer can be played with two emulator windows (on the
//! same PC or over a LAN).
//!
//! Enabled with `--gamekit-host=PORT` on one instance and
//! `--gamekit-join=ADDRESS:PORT` on the other. The picker UI is skipped:
//! `-[GKPeerPickerController show]` connects automatically and then reports
//! the connection to the delegate.
//!
//! Wire format: each `sendData:` call becomes one frame of
//! `[u32 little-endian length][bytes]`.

use crate::frameworks::foundation::{ns_string, NSUInteger};
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr};
use crate::objc::{id, msg, msg_class, nil, objc_classes, release, retain, ClassExports};
use crate::options::GameKitLink;
use crate::Environment;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

const PEER_ID: &str = "touchHLE-peer";
/// GKPeerPickerConnectionTypeNearby
const CONNECTION_TYPE_NEARBY: NSUInteger = 1 << 1;
/// GKPeerStateDisconnected
const GK_PEER_STATE_DISCONNECTED: i32 = 3;

pub struct State {
    listener: Option<TcpListener>,
    stream: Option<TcpStream>,
    rbuf: Vec<u8>,
    next_connect_attempt: Option<Instant>,
    picker: id,
    picker_delegate: id,
    picker_waiting: bool,
    session: id,
    session_delegate: id,
    handler: id,
    handler_context: MutVoidPtr,
}
impl Default for State {
    fn default() -> Self {
        State {
            listener: None,
            stream: None,
            rbuf: Vec::new(),
            next_connect_attempt: None,
            picker: nil,
            picker_delegate: nil,
            picker_waiting: false,
            session: nil,
            session_delegate: nil,
            handler: nil,
            handler_context: MutVoidPtr::null(),
        }
    }
}

fn state(env: &mut Environment) -> &mut State {
    &mut env.framework_state.game_kit.link
}

fn configure_stream(stream: &TcpStream) {
    let _ = stream.set_nodelay(true);
    let _ = stream.set_nonblocking(true);
}

/// Start listening/connecting according to the options.
fn start_link(env: &mut Environment) -> bool {
    let Some(link) = env.options.gamekit_link.clone() else {
        log!("GameKit: no --gamekit-host= or --gamekit-join= option given, peer picker will be cancelled.");
        return false;
    };
    let st = state(env);
    match link {
        GameKitLink::Host(port) => {
            if st.listener.is_none() {
                match TcpListener::bind(("0.0.0.0", port)) {
                    Ok(l) => {
                        let _ = l.set_nonblocking(true);
                        log!("GameKit: hosting, waiting for the other player on port {}", port);
                        st.listener = Some(l);
                    }
                    Err(e) => {
                        log!("GameKit: could not listen on port {}: {}", port, e);
                        return false;
                    }
                }
            }
        }
        GameKitLink::Join(_) => {
            st.next_connect_attempt = Some(Instant::now());
            log!("GameKit: joining {:?}", link);
        }
    }
    true
}

fn close_stream(env: &mut Environment) {
    let st = state(env);
    if let Some(s) = st.stream.take() {
        let _ = s.shutdown(std::net::Shutdown::Both);
    }
    st.rbuf.clear();
}

/// Called from the main run loop each iteration.
pub fn handle_link(env: &mut Environment) {
    // Establish connection if needed.
    if state(env).stream.is_none() {
        let join_addr: Option<SocketAddr> = match env.options.gamekit_link {
            Some(GameKitLink::Join(addr)) => Some(addr),
            _ => None,
        };
        let st = state(env);
        if let Some(listener) = &st.listener {
            if let Ok((s, addr)) = listener.accept() {
                log!("GameKit: other player connected from {}", addr);
                configure_stream(&s);
                st.stream = Some(s);
            }
        } else if let (Some(addr), Some(when)) = (join_addr, st.next_connect_attempt) {
            if Instant::now() >= when {
                match TcpStream::connect_timeout(&addr, Duration::from_millis(100)) {
                    Ok(s) => {
                        log!("GameKit: connected to host {}", addr);
                        configure_stream(&s);
                        st.stream = Some(s);
                        st.next_connect_attempt = None;
                    }
                    Err(_) => {
                        st.next_connect_attempt = Some(Instant::now() + Duration::from_millis(500));
                    }
                }
            }
        }
    }

    if state(env).stream.is_none() {
        return;
    }

    // Report the connection to the picker delegate once.
    if state(env).picker_waiting && state(env).session != nil {
        let st = state(env);
        st.picker_waiting = false;
        let (picker, delegate, session) = (st.picker, st.picker_delegate, st.session);
        let peer = ns_string::get_static_str(env, PEER_ID);
        log!("GameKit: reporting connection to the app");
        () = msg![env; delegate peerPickerController:picker didConnectPeer:peer toSession:session];
    }

    // Read incoming data.
    let mut disconnected = false;
    {
        let st = state(env);
        let stream = st.stream.as_mut().unwrap();
        let mut buf = [0u8; 4096];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => {
                    disconnected = true;
                    break;
                }
                Ok(n) => st.rbuf.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(_) => {
                    disconnected = true;
                    break;
                }
            }
        }
    }

    // Dispatch complete frames (only once the app has a receive handler).
    loop {
        let st = state(env);
        if st.handler == nil || st.rbuf.len() < 4 {
            break;
        }
        let len = u32::from_le_bytes(st.rbuf[..4].try_into().unwrap()) as usize;
        if st.rbuf.len() < 4 + len {
            break;
        }
        let frame: Vec<u8> = st.rbuf.drain(..4 + len).skip(4).collect();
        let (handler, session, context) = (st.handler, st.session, st.handler_context);

        let len32: NSUInteger = frame.len().try_into().unwrap();
        let tmp = env.mem.alloc(len32.max(1));
        env.mem
            .bytes_at_mut(tmp.cast(), len32)
            .copy_from_slice(&frame);
        let data: id = msg_class![env; NSData dataWithBytes:(tmp.cast_const()) length:len32];
        env.mem.free(tmp);
        let peer = ns_string::get_static_str(env, PEER_ID);
        () = msg![env; handler receiveData:data fromPeer:peer inSession:session context:context];
    }

    if disconnected {
        log!("GameKit: other player disconnected");
        close_stream(env);
        let st = state(env);
        let (delegate, session) = (st.session_delegate, st.session);
        if delegate != nil && session != nil {
            let peer = ns_string::get_static_str(env, PEER_ID);
            () = msg![env; delegate session:session peer:peer didChangeState:GK_PEER_STATE_DISCONNECTED];
        }
        // Allow reconnecting for a rematch.
        if let Some(GameKitLink::Join(_)) = env.options.gamekit_link {
            state(env).next_connect_attempt = None;
        }
    }
}

fn send_frame(env: &mut Environment, bytes: &[u8]) -> bool {
    let st = state(env);
    let Some(stream) = st.stream.as_mut() else {
        return false;
    };
    let mut out = Vec::with_capacity(bytes.len() + 4);
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    let mut written = 0;
    while written < out.len() {
        match stream.write(&out[written..]) {
            Ok(0) => return false,
            Ok(n) => written += n,
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(_) => return false,
        }
    }
    true
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GKPeerPickerController: NSObject

- (id)init { this }

- (())setDelegate:(id)delegate {
    state(env).picker_delegate = delegate;
}
- (id)delegate {
    state(env).picker_delegate
}
- (())setConnectionTypesMask:(NSUInteger)_mask {}
- (NSUInteger)connectionTypesMask { CONNECTION_TYPE_NEARBY }
- (bool)isVisible { state(env).picker_waiting }

- (())show {
    let delegate = state(env).picker_delegate;
    if !start_link(env) {
        () = msg![env; delegate peerPickerControllerDidCancel:this];
        return;
    }
    let old_picker = state(env).picker;
    retain(env, this);
    if old_picker != nil {
        release(env, old_picker);
    }
    state(env).picker = this;

    let session: id = msg![env; delegate peerPickerController:this
                                     sessionForConnectionType:CONNECTION_TYPE_NEARBY];
    let session = if session == nil {
        let s: id = msg_class![env; GKSession alloc];
        let name = ns_string::get_static_str(env, "touchHLE");
        msg![env; s initWithSessionID:nil displayName:name sessionMode:2i32]
    } else {
        retain(env, session)
    };
    let old_session = state(env).session;
    if old_session != nil && old_session != session {
        release(env, old_session);
    }
    state(env).session = session;
    state(env).picker_waiting = true;
}

- (())dismiss {
    state(env).picker_waiting = false;
}

@end

@implementation GKSession: NSObject

- (id)initWithSessionID:(id)_session_id
            displayName:(id)_name
            sessionMode:(i32)_mode {
    this
}

- (())setDataReceiveHandler:(id)handler withContext:(MutVoidPtr)context {
    let st = state(env);
    st.handler = handler;
    st.handler_context = context;
    if handler == nil && st.session == this {
        // nothing else to do
    }
}

- (())setDelegate:(id)delegate {
    state(env).session_delegate = delegate;
}
- (id)delegate {
    state(env).session_delegate
}

- (())setAvailable:(bool)_available {}
- (bool)isAvailable { true }
- (id)peerID {
    ns_string::get_static_str(env, "touchHLE-self")
}
- (id)sessionID {
    ns_string::get_static_str(env, "touchHLE")
}
- (id)displayName {
    ns_string::get_static_str(env, "touchHLE")
}
- (id)displayNameForPeer:(id)_peer {
    ns_string::get_static_str(env, "Player")
}
- (i32)sessionMode { 2 }

- (bool)sendData:(id)data
         toPeers:(id)_peers
    withDataMode:(i32)_mode
           error:(MutPtr<id>)error {
    let bytes: ConstVoidPtr = msg![env; data bytes];
    let length: NSUInteger = msg![env; data length];
    let host_bytes = env.mem.bytes_at(bytes.cast(), length).to_vec();
    let ok = send_frame(env, &host_bytes);
    if !error.is_null() {
        env.mem.write(error, nil);
    }
    ok
}

- (bool)sendDataToAllPeers:(id)data
              withDataMode:(i32)mode
                     error:(MutPtr<id>)error {
    msg![env; this sendData:data toPeers:nil withDataMode:mode error:error]
}

- (())disconnectFromAllPeers {
    close_stream(env);
}

- (())disconnectPeerFromAllPeers:(id)_peer {
    close_stream(env);
}

@end

};
