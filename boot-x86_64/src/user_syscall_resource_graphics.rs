use super::*;

pub(super) fn dispatch_resource_graphics_syscall(
    frame: &SyscallFrame,
) -> Option<Result<usize, Errno>> {
    let result = match frame.number {
        SYS_INSPECT_DEVICE => inspect_device_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeDeviceRecord,
        ),
        SYS_INSPECT_DRIVER => inspect_driver_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeDriverRecord,
        ),
        SYS_CREATE_DOMAIN => create_domain_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_CREATE_RESOURCE => {
            create_resource_syscall(frame.arg0, frame.arg1 as u32, frame.arg2, frame.arg3)
        }
        SYS_CREATE_BUS_PEER => create_bus_peer_syscall(frame.arg0, frame.arg1, frame.arg2),
        SYS_CREATE_BUS_ENDPOINT => {
            create_bus_endpoint_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3)
        }
        SYS_ATTACH_BUS_PEER => attach_bus_peer_syscall(frame.arg0, frame.arg1, frame.arg2 as u64),
        SYS_DETACH_BUS_PEER => detach_bus_peer_syscall(frame.arg0, frame.arg1),
        SYS_CREATE_CONTRACT => create_contract_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as u32,
            frame.arg3,
            frame.arg4,
        ),
        SYS_BIND_PROCESS_CONTRACT => bind_process_contract_syscall(frame.arg0),
        SYS_LIST_DOMAINS => list_domains_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_DOMAIN => {
            inspect_domain_syscall(frame.arg0, frame.arg1 as *mut NativeDomainRecord)
        }
        SYS_LIST_RESOURCES => list_resources_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_RESOURCE => {
            inspect_resource_syscall(frame.arg0, frame.arg1 as *mut NativeResourceRecord)
        }
        SYS_LIST_BUS_PEERS => list_bus_peers_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_BUS_PEER => {
            inspect_bus_peer_syscall(frame.arg0, frame.arg1 as *mut NativeBusPeerRecord)
        }
        SYS_LIST_BUS_ENDPOINTS => list_bus_endpoints_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_BUS_ENDPOINT => {
            inspect_bus_endpoint_syscall(frame.arg0, frame.arg1 as *mut NativeBusEndpointRecord)
        }
        SYS_LIST_CONTRACTS => list_contracts_syscall(frame.arg0 as *mut u64, frame.arg1),
        SYS_INSPECT_CONTRACT => {
            syscall_trace(format_args!(
                "ngos/x86_64: dispatch inspect_contract id={} out={:p}\n",
                frame.arg0, frame.arg1 as *mut NativeContractRecord
            ));
            inspect_contract_syscall(frame.arg0, frame.arg1 as *mut NativeContractRecord)
        }
        SYS_GET_DOMAIN_NAME => {
            get_domain_name_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_RESOURCE_NAME => {
            get_resource_name_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_GET_CONTRACT_LABEL => {
            get_contract_label_syscall(frame.arg0, frame.arg1 as *mut u8, frame.arg2)
        }
        SYS_SET_CONTRACT_STATE => set_contract_state_syscall(frame.arg0, frame.arg1 as u32),
        SYS_INVOKE_CONTRACT => {
            syscall_trace(format_args!(
                "ngos/x86_64: dispatch invoke_contract id={}\n",
                frame.arg0
            ));
            invoke_contract_syscall(frame.arg0)
        }
        SYS_RELEASE_RESOURCE => release_resource_syscall(frame.arg0),
        SYS_TRANSFER_RESOURCE => transfer_resource_syscall(frame.arg0, frame.arg1),
        SYS_SET_RESOURCE_POLICY => set_resource_policy_syscall(frame.arg0, frame.arg1 as u32),
        SYS_SET_RESOURCE_GOVERNANCE => {
            set_resource_governance_syscall(frame.arg0, frame.arg1 as u32)
        }
        SYS_SET_RESOURCE_CONTRACT_POLICY => {
            set_resource_contract_policy_syscall(frame.arg0, frame.arg1 as u32)
        }
        SYS_SET_RESOURCE_ISSUER_POLICY => {
            set_resource_issuer_policy_syscall(frame.arg0, frame.arg1 as u32)
        }
        SYS_SET_RESOURCE_STATE => set_resource_state_syscall(frame.arg0, frame.arg1 as u32),
        SYS_ACQUIRE_RESOURCE => acquire_resource_syscall(frame.arg0),
        SYS_CLAIM_RESOURCE => {
            claim_resource_syscall(frame.arg0, frame.arg1 as *mut NativeResourceClaimRecord)
        }
        SYS_RELEASE_CLAIMED_RESOURCE => {
            syscall_trace(format_args!(
                "ngos/x86_64: dispatch release_claimed_resource contract={} out={:p}\n",
                frame.arg0, frame.arg1 as *mut NativeResourceReleaseRecord
            ));
            release_claimed_resource_syscall(
                frame.arg0,
                frame.arg1 as *mut NativeResourceReleaseRecord,
            )
        }
        SYS_LIST_RESOURCE_WAITERS => {
            list_resource_waiters_syscall(frame.arg0, frame.arg1 as *mut u64, frame.arg2)
        }
        SYS_CANCEL_RESOURCE_CLAIM => {
            cancel_resource_claim_syscall(frame.arg0, frame.arg1 as *mut NativeResourceCancelRecord)
        }
        SYS_PUBLISH_BUS_MESSAGE => {
            publish_bus_message_syscall(frame.arg0, frame.arg1, frame.arg2 as *const u8, frame.arg3)
        }
        SYS_RECEIVE_BUS_MESSAGE => {
            receive_bus_message_syscall(frame.arg0, frame.arg1, frame.arg2 as *mut u8, frame.arg3)
        }
        SYS_WATCH_BUS_EVENTS => watch_bus_events_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const NativeBusEventWatchConfig,
        ),
        SYS_REMOVE_BUS_EVENTS => {
            remove_bus_events_syscall(frame.arg0, frame.arg1, frame.arg2 as u64)
        }
        SYS_BOOT_REPORT => boot_report_syscall(
            frame.arg0 as u32,
            frame.arg1 as u32,
            frame.arg2 as i32,
            frame.arg3 as u64,
        ),
        SYS_INSPECT_DEVICE_REQUEST => {
            inspect_device_request_syscall(frame.arg0, frame.arg1 as *mut NativeDeviceRequestRecord)
        }
        SYS_INSPECT_GPU_DISPLAY => inspect_gpu_display_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeGpuDisplayRecord,
        ),
        SYS_INSPECT_GPU_SCANOUT => inspect_gpu_scanout_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeGpuScanoutRecord,
        ),
        SYS_PRESENT_GPU_FRAME => {
            present_gpu_frame_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3)
        }
        SYS_READ_GPU_SCANOUT_FRAME => read_gpu_scanout_frame_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut u8,
            frame.arg3,
        ),
        _ => return None,
    };
    Some(result)
}
