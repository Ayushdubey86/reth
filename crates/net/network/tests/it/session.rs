//! Session tests

use futures::StreamExt;
use reth_eth_wire::EthVersion;
use reth_network::{
    test_utils::{NetworkEventStream, PeerConfig, Testnet},
    NetworkEvent, NetworkEventListenerProvider,
};
use reth_network_api::{
    events::{PeerEvent, SessionInfo},
    NetworkInfo, Peers,
};
use reth_storage_api::noop::NoopProvider;

#[tokio::test(flavor = "multi_thread")]
async fn test_session_established_with_highest_version() {
    reth_tracing::init_test_tracing();

    let net = Testnet::create(2).await;
    net.for_each(|peer| assert_eq!(0, peer.num_peers()));

    let mut handles = net.handles();
    let handle0 = handles.next().unwrap();
    let handle1 = handles.next().unwrap();
    drop(handles);

    let handle = net.spawn();

    let mut events = handle0.event_listener().take(2);
    handle0.add_peer(*handle1.peer_id(), handle1.local_addr());

    while let Some(event) = events.next().await {
        match event {
            NetworkEvent::Peer(PeerEvent::PeerAdded(peer_id)) => {
                assert_eq!(handle1.peer_id(), &peer_id);
            }
            NetworkEvent::ActivePeerSession { info, .. } => {
                let SessionInfo { peer_id, status, .. } = info;
                assert_eq!(handle1.peer_id(), &peer_id);
                assert_eq!(status.version, EthVersion::Eth68);
            }
            ev => {
                panic!("unexpected event {ev:?}")
            }
        }
    }
    handle.terminate().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_session_established_with_different_capability() {
    reth_tracing::init_test_tracing();

    let mut net = Testnet::create(1).await;

    let p1 = PeerConfig::with_protocols(NoopProvider::default(), Some(EthVersion::Eth66.into()));
    net.add_peer_with_config(p1).await.unwrap();

    net.for_each(|peer| assert_eq!(0, peer.num_peers()));

    let mut handles = net.handles();
    let handle0 = handles.next().unwrap();
    let handle1 = handles.next().unwrap();
    drop(handles);

    let handle = net.spawn();

    let mut events = handle0.event_listener().take(2);
    handle0.add_peer(*handle1.peer_id(), handle1.local_addr());

    while let Some(event) = events.next().await {
        match event {
            NetworkEvent::Peer(PeerEvent::PeerAdded(peer_id)) => {
                assert_eq!(handle1.peer_id(), &peer_id);
            }
            NetworkEvent::ActivePeerSession { info, .. } => {
                let SessionInfo { peer_id, status, .. } = info;
                assert_eq!(handle1.peer_id(), &peer_id);
                assert_eq!(status.version, EthVersion::Eth66);
            }
            ev => {
                panic!("unexpected event: {ev:?}")
            }
        }
    }

    handle.terminate().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_capability_version_mismatch() {
    reth_tracing::init_test_tracing();

    let mut net = Testnet::create(0).await;

    let p0 = PeerConfig::with_protocols(NoopProvider::default(), Some(EthVersion::Eth66.into()));
    net.add_peer_with_config(p0).await.unwrap();

    let p1 = PeerConfig::with_protocols(NoopProvider::default(), Some(EthVersion::Eth67.into()));
    net.add_peer_with_config(p1).await.unwrap();

    net.for_each(|peer| assert_eq!(0, peer.num_peers()));

    let mut handles = net.handles();
    let handle0 = handles.next().unwrap();
    let handle1 = handles.next().unwrap();
    drop(handles);

    let handle = net.spawn();

    let events = handle0.event_listener();

    let mut event_stream = NetworkEventStream::new(events);

    handle0.add_peer(*handle1.peer_id(), handle1.local_addr());

    let added_peer_id = event_stream.peer_added().await.unwrap();
    assert_eq!(added_peer_id, *handle1.peer_id());

    // peer with mismatched capability version should fail to connect and be removed.
    let removed_peer_id = event_stream.peer_removed().await.unwrap();
    assert_eq!(removed_peer_id, *handle1.peer_id());

    handle.terminate().await;
}
