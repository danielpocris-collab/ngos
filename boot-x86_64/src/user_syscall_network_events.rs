use super::*;

pub(super) fn dispatch_network_event_syscall(frame: &SyscallFrame) -> Option<Result<usize, Errno>> {
    let result = match frame.number {
        SYS_CREATE_EVENT_QUEUE => create_event_queue_syscall(frame.arg0 as u32),
        SYS_WAIT_EVENT_QUEUE => {
            wait_event_queue_syscall(frame.arg0, frame.arg1 as *mut NativeEventRecord, frame.arg2)
        }
        SYS_WATCH_RESOURCE_EVENTS => watch_resource_events_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const NativeResourceEventWatchConfig,
        ),
        SYS_REMOVE_RESOURCE_EVENTS => {
            remove_resource_events_syscall(frame.arg0, frame.arg1, frame.arg2 as u64)
        }
        SYS_WATCH_NET_EVENTS => watch_network_events_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3,
            frame.arg4,
            frame.arg5 as *const NativeNetworkEventWatchConfig,
        ),
        SYS_WATCH_VFS_EVENTS => watch_vfs_events_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3 as *const NativeVfsEventWatchConfig,
        ),
        SYS_WATCH_VFS_EVENTS_AT => watch_vfs_events_at_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3,
            frame.arg4 as *const NativeVfsEventWatchConfig,
        ),
        SYS_REMOVE_NET_EVENTS => remove_network_events_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3,
            frame.arg4,
            frame.arg5 as u64,
        ),
        SYS_REMOVE_VFS_EVENTS => {
            remove_vfs_events_syscall(frame.arg0, frame.arg1, frame.arg2, frame.arg3 as u64)
        }
        SYS_REMOVE_VFS_EVENTS_AT => remove_vfs_events_at_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3,
            frame.arg4 as u64,
        ),
        SYS_CONFIGURE_NETIF_IPV4 => configure_network_interface_ipv4_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const NativeNetworkInterfaceConfig,
        ),
        SYS_BIND_UDP_SOCKET => bind_udp_socket_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2,
            frame.arg3,
            frame.arg4 as *const NativeUdpBindConfig,
        ),
        SYS_INSPECT_NETIF => inspect_network_interface_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeNetworkInterfaceRecord,
        ),
        SYS_INSPECT_NETSOCK => inspect_network_socket_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut NativeNetworkSocketRecord,
        ),
        SYS_SET_NETIF_LINK_STATE => set_network_link_state_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const NativeNetworkLinkStateConfig,
        ),
        SYS_CONFIGURE_NETIF_ADMIN => configure_network_interface_admin_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const NativeNetworkAdminConfig,
        ),
        SYS_CONNECT_UDP_SOCKET => connect_udp_socket_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const NativeUdpConnectConfig,
        ),
        SYS_SENDTO_UDP_SOCKET => sendto_udp_socket_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *const NativeUdpSendToConfig,
            frame.arg3 as *const u8,
            frame.arg4,
        ),
        SYS_RECVFROM_UDP_SOCKET => recvfrom_udp_socket_syscall(
            frame.arg0,
            frame.arg1,
            frame.arg2 as *mut u8,
            frame.arg3,
            frame.arg4 as *mut NativeUdpRecvMeta,
        ),
        SYS_COMPLETE_NET_TX => complete_network_tx_syscall(frame.arg0, frame.arg1, frame.arg2),
        _ => return None,
    };
    Some(result)
}
