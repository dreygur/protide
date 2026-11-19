/// Makes `ring` the process wide rustls provider before a `wss://` connect.
///
/// With `p2p-sync`, libp2p enables rustls' `aws-lc-rs` next to the `ring`
/// the HTTP stack uses. With both compiled in, rustls refuses to choose and
/// panics the first time a client config is built without an explicit
/// provider, which is what tungstenite does. Installing fails harmlessly
/// once a provider is set, so every connect may call this.
pub(crate) fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
