pub(super) use ngos_shell_nextmind::{
    NextMindAgentState, NextMindAutoState, NextMindDecisionReport, nextmind_drain_auto_events,
    try_handle_nextmind_agent_command,
};
#[cfg(test)]
pub(super) use ngos_shell_nextmind::{
    nextmind_auto_summary, nextmind_auto_triggered, nextmind_channel_for_metrics,
    nextmind_explain_last, nextmind_metrics_score, nextmind_subscribe_auto_streams,
    test_nextmind_metrics,
};
