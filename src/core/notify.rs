//! Desktop notifications via the freedesktop `org.freedesktop.Notifications`
//! D-Bus service.
//!
//! Used to surface failures the user would otherwise never see — most notably a
//! recording that dies in its background thread after the UI has already flipped
//! to the "recording" state.

use std::collections::HashMap;
use zbus::zvariant::Value;

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, &Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

/// Show an error notification.
///
/// Fire-and-forget: runs on its own thread with a dedicated runtime so it is
/// safe to call from any context (the async app loop, a std recording thread,
/// etc.) and never blocks the caller. Failure to deliver is logged, not
/// propagated — a missing notification daemon shouldn't take anything down.
pub fn notify_error(summary: impl Into<String>, body: impl Into<String>) {
    let summary = summary.into();
    let body = body.into();
    std::thread::spawn(move || {
        if let Err(e) = send(&summary, &body) {
            log::warn!("Could not show desktop notification: {e}");
        }
    });
}

fn send(summary: &str, body: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let conn = zbus::Connection::session().await?;
        let proxy = NotificationsProxy::new(&conn).await?;
        let hints: HashMap<&str, &Value<'_>> = HashMap::new();
        proxy
            .notify(
                "SnapPea",
                0,             // replaces_id: 0 = new notification
                "dialog-error",
                summary,
                body,
                &[],           // no actions
                hints,
                5000,          // expire after 5s
            )
            .await?;
        Ok::<(), zbus::Error>(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sends a real notification; requires a running notification daemon, so it's
    // ignored by default. Run with: `cargo test notify_smoke -- --ignored`.
    #[test]
    #[ignore]
    fn notify_smoke() {
        send("SnapPea test", "If you can see this, notifications work.")
            .expect("notification should be delivered");
    }
}
