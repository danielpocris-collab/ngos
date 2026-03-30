use super::*;
use ngos_gfx_translate::{DrawOp, FrameScript, RgbaColor};

/// A simplified implementation of VkQueueSubmit for ngos.
/// Translates Vulkan command buffers into semantic FrameScripts.
pub fn vk_queue_submit<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    buffer_id: u64,
) -> Result<(), ExitCode> {
    // In ngos, we don't just send raw bytes.
    // We notify the NvidiaGspAgent that a new frame is ready for neural infusion.
    
    let mut game_sessions = Vec::<GameCompatSession>::new(); // In a real app, this is global/passed
    
    // Call the shell-level submit (which we've already optimized for hardware)
    // In a real ICD, this would call SYS_SUBMIT_GPU_BUFFER
    game_submit_frame_by_id(runtime, device_path, buffer_id)
}

fn game_submit_frame_by_id<B: SyscallBackend>(
    runtime: &Runtime<B>,
    device_path: &str,
    buffer_id: u64,
) -> Result<(), ExitCode> {
    // This maps to our Kernel hardware path
    runtime.submit_graphics_buffer(device_path, buffer_id)
        .map(|_| ())
        .map_err(|_| 300)
}

/// Vulkan Device creation in ngos.
/// This triggers the 'setup_gpu_agent' in the kernel.
pub fn vk_create_device<B: SyscallBackend>(
    runtime: &Runtime<B>,
    _physical_device_id: u32,
) -> Result<String, ExitCode> {
    let path = "/dev/gpu0";
    // The mere act of opening/inspecting can trigger agent activation in our kernel
    if let Ok(_) = runtime.inspect_device(path) {
        Ok(String::from(path))
    } else {
        Err(301)
    }
}

/// NGOS Specific Extension: VK_NGOS_neural_infusion
/// Injects semantic metadata into the Vulkan pipeline.
pub fn vk_cmd_inject_semantics_ngos<B: SyscallBackend>(
    runtime: &Runtime<B>,
    _device_path: &str,
    label: &str,
) -> Result<(), ExitCode> {
    // This will map to a new syscall or a specific GPU Control opcode
    // For now, we simulate the injection into the neural agent
    let payload = label.as_bytes();
    // RPC ID 0xe503 is NV_RPC_ID_DLSS5_INJECT_SEMANTICS
    runtime.submit_gpu_control_command(0xe503, payload)
        .map(|_| ())
        .map_err(|_| 305)
}
