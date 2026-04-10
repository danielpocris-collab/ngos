use ngos_game_compat_runtime::{GameCompatManifest, GameSessionPlan};

use crate::GameCompatSession;

pub fn build_game_session_identity(
    session: &mut GameCompatSession,
    manifest: &GameCompatManifest,
    plan: &GameSessionPlan,
    pid: u64,
    domain_id: usize,
) {
    session.target = manifest.target;
    session.title = manifest.title.clone();
    session.slug = manifest.slug.clone();
    session.pid = pid;
    session.domain_id = domain_id;
    session.process_name = plan.process_name.clone();
    session.executable_path = plan.executable_path.clone();
    session.working_dir = plan.working_dir.clone();
    session.prefix_path = manifest.shims.prefix.clone();
    session.saves_path = manifest.shims.saves.clone();
    session.cache_path = manifest.shims.cache.clone();
}
