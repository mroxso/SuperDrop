use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use qrcode::{render::unicode, QrCode};
use std::time::{Duration, Instant};
use superdrop_core::{QrSession, SERVICE};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() != Some("activate") {
        eprintln!("Usage: superdrop activate\nExperimental QR activation/discovery only; file transport is not implemented.");
        std::process::exit(2);
    }
    let session = QrSession::new();
    let url = session.url().ok_or("expired session")?;
    println!(
        "Scan with Android's camera on the same LAN. Expires in 120 seconds.\n{}",
        QrCode::new(url.as_bytes())?
            .render::<unicode::Dense1x2>()
            .build()
    );
    let daemon = ServiceDaemon::new()?;
    let events = daemon.browse(SERVICE)?;
    let start = Instant::now();
    while !session.expired() {
        if let Ok(ServiceEvent::ServiceResolved(info)) =
            events.recv_timeout(Duration::from_millis(250))
        {
            if let Some(value) = info.get_property_val_str("n") {
                if let Ok(bytes) = URL_SAFE_NO_PAD.decode(value.trim_end_matches('=')) {
                    if let Some(name) = session.match_endpoint(&bytes) {
                        println!("QR-bound receiver: {name:?}, port {}, {} addresses. Transport not yet implemented.", info.get_port(), info.get_addresses().len());
                        daemon.stop_browse(SERVICE)?;
                        daemon.shutdown()?;
                        return Ok(());
                    }
                }
            }
        }
    }
    daemon.stop_browse(SERVICE)?;
    daemon.shutdown()?;
    Err(format!(
        "No QR-bound receiver within {} seconds; check LAN isolation, VPN and firewall",
        start.elapsed().as_secs()
    )
    .into())
}
