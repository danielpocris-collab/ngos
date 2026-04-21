use alloc::format;
use alloc::string::String;

use ngos_user_abi::{ExitCode, SyscallBackend};
use ngos_user_runtime::Runtime;

use crate::write_line;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameQualityReport {
    pub title: String,
    pub slug: String,
    pub frames_submitted: usize,
    pub frames_presented: usize,
    pub max_latency: u64,
    pub avg_latency: u64,
    pub budget_hits: usize,
    pub backpressure_events: usize,
}

pub fn render_quality_report<B: SyscallBackend>(
    runtime: &Runtime<B>,
    report: &GameQualityReport,
) -> Result<(), ExitCode> {
    write_line(runtime, "== GAME QUALITY REPORT ==")?;
    write_line(runtime, &format!("title: {}", report.title))?;
    write_line(runtime, &format!("slug:  {}", report.slug))?;
    write_line(
        runtime,
        &format!("frames_submitted: {}", report.frames_submitted),
    )?;
    write_line(
        runtime,
        &format!("frames_presented: {}", report.frames_presented),
    )?;
    write_line(
        runtime,
        &format!("max_latency_ticks: {}", report.max_latency),
    )?;
    write_line(
        runtime,
        &format!("avg_latency_ticks: {}", report.avg_latency),
    )?;
    write_line(runtime, &format!("budget_hits: {}", report.budget_hits))?;
    write_line(
        runtime,
        &format!("backpressure_events: {}", report.backpressure_events),
    )?;

    let quality_score = if report.frames_submitted > 0 {
        let base = (report.frames_presented as f32 / report.frames_submitted as f32) * 100.0;
        let penalty = (report.budget_hits as f32 * 5.0) + (report.backpressure_events as f32 * 2.0);
        (base - penalty).max(0.0)
    } else {
        0.0
    };

    write_line(runtime, &format!("quality_score: {:.2}", quality_score))?;
    write_line(runtime, "== END REPORT ==")?;
    Ok(())
}
