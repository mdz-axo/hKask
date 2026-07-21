//! Window catalog and factory — canonical list of window kinds and constructors.

use std::sync::Arc;

use crate::bridges::{
    BackupDataBridge, CompaniesDataBridge, ConfigDataBridge, DocprocDataBridge, KanbanDataBridge,
    MatrixDataBridge, MediaDataBridge, MemoryDataBridge, RegistryDataBridge, ReplicaDataBridge,
    ResearchDataBridge, ScenariosDataBridge, SkillsDataBridge, TrainingDataBridge,
    WalletDataBridge,
};
use crate::repl_bridge::{ReplBridge, SessionBridge, SettingsBridge, SystemBridge};
use crate::window::{Window, WindowId, WindowKind};
use crate::windows::backup::BackupWindow;
use crate::windows::chat::ChatWindow;
use crate::windows::companies::CompaniesWindow;
use crate::windows::configuration::ConfigurationWindow;
use crate::windows::docproc::DocprocWindow;
use crate::windows::editor::EditorWindow;
use crate::windows::kanban::KanbanWindow;
use crate::windows::matrix::MatrixWindow;
use crate::windows::media::MediaWindow;
use crate::windows::memory::MemoryWindow;
use crate::windows::pods::PodsWindow;
use crate::windows::reg_monitor::CnsMonitorWindow;
use crate::windows::registry::RegistryWindow;
use crate::windows::replica::ReplicaWindow;
use crate::windows::research::ResearchWindow;
use crate::windows::scenarios::ScenariosWindow;
use crate::windows::skills::SkillsWindow;
use crate::windows::terminal::TerminalWindow;
use crate::windows::training::TrainingWindow;
use crate::windows::wallet::WalletWindow;

/// Macro: window with optional bridge wiring.
/// `mk_bridge!(FooWindow, ctx.wallet_bridge.clone(), with_wallet_bridge, id, bridge)`
macro_rules! mk_bridge {
    ($ctor:ident, $b:expr, $method:ident, $id:expr, $br:expr) => {{
        let mut w = $ctor::new($id, $br);
        if let Some(b) = $b {
            w = w.$method(b);
        }
        Box::new(w)
    }};
}

pub(crate) fn window_kinds() -> Vec<WindowKind> {
    WindowKind::META.iter().map(|(k, ..)| *k).collect()
}

pub(crate) fn window_kind_from_title(title: &str) -> Option<WindowKind> {
    WindowKind::META
        .iter()
        .find(|(_, t, ..)| *t == title)
        .map(|(k, ..)| *k)
}

/// All bridge dependencies for window construction.
pub(crate) struct WindowBridges {
    pub system_bridge: Arc<dyn SystemBridge>,
    pub repl_bridge: Arc<dyn ReplBridge>,
    pub settings_bridge: Option<Arc<dyn SettingsBridge>>,
    pub session_bridge: Option<Arc<dyn SessionBridge>>,
    pub wallet_bridge: Option<Arc<dyn WalletDataBridge>>,
    pub config_bridge: Option<Arc<dyn ConfigDataBridge>>,
    pub backup_bridge: Option<Arc<dyn BackupDataBridge>>,
    pub registry_bridge: Option<Arc<dyn RegistryDataBridge>>,
    pub memory_bridge: Option<Arc<dyn MemoryDataBridge>>,
    pub kanban_bridge: Option<Arc<dyn KanbanDataBridge>>,
    pub matrix_bridge: Option<Arc<dyn MatrixDataBridge>>,
    pub media_bridge: Option<Arc<dyn MediaDataBridge>>,
    pub training_bridge: Option<Arc<dyn TrainingDataBridge>>,
    pub companies_bridge: Option<Arc<dyn CompaniesDataBridge>>,
    pub research_bridge: Option<Arc<dyn ResearchDataBridge>>,
    pub docproc_bridge: Option<Arc<dyn DocprocDataBridge>>,
    pub replica_bridge: Option<Arc<dyn ReplicaDataBridge>>,
    pub skills_bridge: Option<Arc<dyn SkillsDataBridge>>,
    pub scenarios_bridge: Option<Arc<dyn ScenariosDataBridge>>,
}

pub(crate) fn create_window(
    kind: WindowKind,
    id: WindowId,
    ctx: &WindowBridges,
) -> Box<dyn Window> {
    let bridge = ctx.repl_bridge.clone();

    match kind {
        WindowKind::CnsMonitor => Box::new(CnsMonitorWindow::new(id, bridge)),
        WindowKind::Pods => Box::new(PodsWindow::new(id, bridge)),
        WindowKind::Terminal => Box::new(TerminalWindow::new(id, bridge)),
        WindowKind::Editor => Box::new(EditorWindow::new(id, bridge)),

        WindowKind::Wallet => {
            mk_bridge!(
                WalletWindow,
                ctx.wallet_bridge.clone(),
                with_wallet_bridge,
                id,
                bridge
            )
        }
        WindowKind::Registry => {
            mk_bridge!(
                RegistryWindow,
                ctx.registry_bridge.clone(),
                with_registry_bridge,
                id,
                bridge
            )
        }
        WindowKind::Backup => {
            mk_bridge!(
                BackupWindow,
                ctx.backup_bridge.clone(),
                with_backup_bridge,
                id,
                bridge
            )
        }
        WindowKind::Configuration => {
            mk_bridge!(
                ConfigurationWindow,
                ctx.config_bridge.clone(),
                with_config_bridge,
                id,
                bridge
            )
        }
        WindowKind::Training => {
            mk_bridge!(
                TrainingWindow,
                ctx.training_bridge.clone(),
                with_training_bridge,
                id,
                bridge
            )
        }
        WindowKind::Media => {
            mk_bridge!(
                MediaWindow,
                ctx.media_bridge.clone(),
                with_media_bridge,
                id,
                bridge
            )
        }
        WindowKind::Research => {
            mk_bridge!(
                ResearchWindow,
                ctx.research_bridge.clone(),
                with_research_bridge,
                id,
                bridge
            )
        }
        WindowKind::Docproc => {
            mk_bridge!(
                DocprocWindow,
                ctx.docproc_bridge.clone(),
                with_docproc_bridge,
                id,
                bridge
            )
        }
        WindowKind::Replica => {
            mk_bridge!(
                ReplicaWindow,
                ctx.replica_bridge.clone(),
                with_replica_bridge,
                id,
                bridge
            )
        }
        WindowKind::Matrix => {
            mk_bridge!(
                MatrixWindow,
                ctx.matrix_bridge.clone(),
                with_matrix_bridge,
                id,
                bridge
            )
        }
        WindowKind::Memory => {
            mk_bridge!(
                MemoryWindow,
                ctx.memory_bridge.clone(),
                with_memory_bridge,
                id,
                bridge
            )
        }
        WindowKind::Companies => {
            mk_bridge!(
                CompaniesWindow,
                ctx.companies_bridge.clone(),
                with_companies_bridge,
                id,
                bridge
            )
        }
        WindowKind::Kanban => {
            mk_bridge!(
                KanbanWindow,
                ctx.kanban_bridge.clone(),
                with_kanban_bridge,
                id,
                bridge
            )
        }

        WindowKind::Skills => {
            let mut w = SkillsWindow::new(id, bridge);
            if let Some(b) = ctx.registry_bridge.clone() {
                w = w.with_registry_bridge(b);
            }
            if let Some(b) = ctx.skills_bridge.clone() {
                w = w.with_skills_bridge(b);
            }
            Box::new(w)
        }

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

        WindowKind::Scenarios => {
            mk_bridge!(
                ScenariosWindow,
                ctx.scenarios_bridge.clone(),
                with_scenarios_bridge,
                id,
                bridge
            )
        }
    }
}
