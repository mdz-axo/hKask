//! Window catalog and factory — canonical list of window kinds and constructors.

use std::sync::Arc;

use crate::repl_bridge::{ReplBridge, SessionBridge, SettingsBridge, SystemBridge};
use crate::window::{Window, WindowId, WindowKind};
use crate::windows::chat::ChatWindow;

pub fn window_kinds() -> Vec<WindowKind> {
    // The TUI hosts only the Chat window; all other window kinds were removed.
    vec![WindowKind::Chat]
}

pub fn window_kind_from_title(title: &str) -> Option<WindowKind> {
    match title {
        "Chat" => Some(WindowKind::Chat),
        _ => None,
    }
}

/// All bridge dependencies for window construction.
pub(crate) struct WindowBridges {
    pub system_bridge: Arc<dyn SystemBridge>,
    pub repl_bridge: Arc<dyn ReplBridge>,
    pub settings_bridge: Option<Arc<dyn SettingsBridge>>,
    pub session_bridge: Option<Arc<dyn SessionBridge>>,
}

pub(crate) fn create_window(
    kind: WindowKind,
    id: WindowId,
    ctx: &WindowBridges,
) -> Box<dyn Window> {
    let bridge = ctx.repl_bridge.clone();

    match kind {
        WindowKind::Chat => {
            let mut w = ChatWindow::new(
                id,
                ctx.system_bridge.userpod_name(),
                ctx.system_bridge.model_name(),
                bridge,
            );
            if let Some(b) = ctx.settings_bridge.clone() {
                w = w.with_settings_bridge(b);
            }
            if let Some(b) = ctx.session_bridge.clone() {
                w = w.with_session_bridge(b);
            }
            Box::new(w)
        }
    }
}
