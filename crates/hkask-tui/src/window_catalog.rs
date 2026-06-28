//! Window catalog and factory — canonical list of window kinds and constructors.

use std::sync::Arc;

use crate::bridges::{
    BackupDataBridge, CompaniesDataBridge, ConfigDataBridge, DocprocDataBridge, KanbanDataBridge,
    MatrixDataBridge, MediaDataBridge, MemoryDataBridge, RegistryDataBridge, ReplicaDataBridge,
    ResearchDataBridge, SkillsDataBridge, TrainingDataBridge, WalletDataBridge,
};
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};
use crate::windows::backup::BackupWindow;
use crate::windows::chat::ChatWindow;
use crate::windows::cns_monitor::CnsMonitorWindow;
use crate::windows::companies::CompaniesWindow;
use crate::windows::configuration::ConfigurationWindow;
use crate::windows::curator::CuratorWindow;
use crate::windows::docproc::DocprocWindow;
use crate::windows::editor::EditorWindow;
use crate::windows::kanban::KanbanWindow;
use crate::windows::logo::LogoWindow;
use crate::windows::matrix::MatrixWindow;
use crate::windows::media::MediaWindow;
use crate::windows::memory::MemoryWindow;
use crate::windows::pods::PodsWindow;
use crate::windows::registry::RegistryWindow;
use crate::windows::replica::ReplicaWindow;
use crate::windows::research::ResearchWindow;

use crate::windows::skills::SkillsWindow;
use crate::windows::terminal::TerminalWindow;
use crate::windows::training::TrainingWindow;
use crate::windows::wallet::WalletWindow;

const WINDOW_KINDS: &[WindowKind] = &[
    WindowKind::Chat,
    WindowKind::Curator,
    WindowKind::CnsMonitor,
    WindowKind::Pods,
    WindowKind::Wallet,
    WindowKind::Configuration,
    WindowKind::Registry,
    WindowKind::Skills,
    WindowKind::Backup,
    WindowKind::Kanban,
    WindowKind::Memory,
    WindowKind::Matrix,
    WindowKind::Media,
    WindowKind::Training,
    WindowKind::Terminal,
    WindowKind::Editor,
    WindowKind::Companies,
    WindowKind::Research,
    WindowKind::Docproc,
    WindowKind::Replica,
    WindowKind::Logo,
];

pub(crate) fn window_kinds() -> &'static [WindowKind] {
    WINDOW_KINDS
}

pub(crate) fn window_kind_from_title(title: &str) -> Option<WindowKind> {
    window_kinds()
        .iter()
        .copied()
        .find(|kind| kind.default_title() == title)
}

pub(crate) struct WindowFactoryContext {
    pub service_context: Option<Arc<hkask_services::AgentService>>,
    pub bridge: Arc<dyn ReplBridge>,
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
}

pub(crate) fn create_window(
    kind: WindowKind,
    id: WindowId,
    ctx: &WindowFactoryContext,
) -> Box<dyn Window> {
    let bridge = ctx.bridge.clone();
    let svc = ctx.service_context.clone();
    let wb = ctx.wallet_bridge.clone();
    let cb = ctx.config_bridge.clone();
    let bb = ctx.backup_bridge.clone();
    let rb = ctx.registry_bridge.clone();
    let mb = ctx.memory_bridge.clone();
    let kb = ctx.kanban_bridge.clone();
    let mxb = ctx.matrix_bridge.clone();
    let mdb = ctx.media_bridge.clone();
    let tb = ctx.training_bridge.clone();
    let cpb = ctx.companies_bridge.clone();
    let rsb = ctx.research_bridge.clone();
    let dpb = ctx.docproc_bridge.clone();
    let rpb = ctx.replica_bridge.clone();
    let sb = ctx.skills_bridge.clone();

    match kind {
        WindowKind::CnsMonitor => Box::new(CnsMonitorWindow::new(id, bridge)),
        WindowKind::Pods => Box::new(PodsWindow::new(id, bridge)),
        WindowKind::Wallet => {
            let mut w = WalletWindow::new(id, bridge);
            if let Some(b) = wb {
                w = w.with_wallet_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Registry => {
            let mut w = RegistryWindow::new(id, bridge);
            if let Some(b) = rb {
                w = w.with_registry_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Backup => {
            let mut w = BackupWindow::new(id, bridge);
            if let Some(b) = bb {
                w = w.with_backup_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Curator => Box::new(CuratorWindow::new(id, bridge)),
        WindowKind::Configuration => {
            let mut w = ConfigurationWindow::new(id, bridge);
            if let Some(b) = cb {
                w = w.with_config_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Terminal => Box::new(TerminalWindow::new(id, bridge)),
        WindowKind::Editor => Box::new(EditorWindow::new(id, bridge)),
        WindowKind::Training => {
            let mut w = TrainingWindow::new(id, bridge);
            if let Some(b) = tb {
                w = w.with_training_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Media => {
            let mut w = MediaWindow::new(id, bridge);
            if let Some(b) = mdb {
                w = w.with_media_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Skills => {
            let mut w = SkillsWindow::new(id, bridge);
            if let Some(b) = rb {
                w = w.with_registry_bridge(b);
            }
            if let Some(b) = sb {
                w = w.with_skills_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Research => {
            let mut w = ResearchWindow::new(id, bridge);
            if let Some(b) = rsb {
                w = w.with_research_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Docproc => {
            let mut w = DocprocWindow::new(id, bridge);
            if let Some(b) = dpb {
                w = w.with_docproc_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Replica => {
            let mut w = ReplicaWindow::new(id, bridge);
            if let Some(b) = rpb {
                w = w.with_replica_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Matrix => {
            let mut w = MatrixWindow::new(id, bridge);
            if let Some(b) = mxb {
                w = w.with_matrix_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Memory => {
            let mut w = MemoryWindow::new(id, bridge);
            if let Some(b) = mb {
                w = w.with_memory_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Companies => {
            let mut w = CompaniesWindow::new(id, bridge);
            if let Some(b) = cpb {
                w = w.with_companies_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Kanban => {
            let mut w = KanbanWindow::new(id, bridge);
            if let Some(b) = kb {
                w = w.with_kanban_bridge(b);
            }
            Box::new(w)
        }
        WindowKind::Logo => Box::new(LogoWindow::new(id)),
        WindowKind::Chat => Box::new(ChatWindow::new(
            id,
            ctx.bridge.agent_name(),
            ctx.bridge.model_name(),
            svc,
            bridge,
        )),
    }
}
