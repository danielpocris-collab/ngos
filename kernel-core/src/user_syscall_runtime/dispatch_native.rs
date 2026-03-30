use super::*;

impl KernelRuntime {
    pub(super) fn dispatch_native_model_syscall(
        &mut self,
        caller: ProcessId,
        frame: SyscallFrame,
    ) -> Result<Option<SyscallReturn>, RuntimeError> {
        match frame.number {
            SYS_CREATE_DOMAIN => {
                let parent = if frame.arg0 == 0 {
                    None
                } else {
                    Some(self.find_domain_id_by_raw(frame.arg0 as u64)?)
                };
                let name = match frame_string(self, caller, frame.arg1, frame.arg2) {
                    Ok(name) => name,
                    Err(result) => return Ok(Some(result)),
                };
                let domain = self.create_domain(caller, parent, name)?;
                Ok(Some(SyscallReturn::ok(domain.raw() as usize)))
            }
            SYS_CREATE_RESOURCE => {
                let domain = self.find_domain_id_by_raw(frame.arg0 as u64)?;
                let kind = match decode_native_resource_kind(frame.arg1) {
                    Some(kind) => kind,
                    None => return Ok(Some(SyscallReturn::err(Errno::Inval))),
                };
                let name = match frame_string(self, caller, frame.arg2, frame.arg3) {
                    Ok(name) => name,
                    Err(result) => return Ok(Some(result)),
                };
                let resource = self.create_resource(caller, domain, kind, name)?;
                Ok(Some(SyscallReturn::ok(resource.raw() as usize)))
            }
            SYS_CREATE_CONTRACT => {
                let domain = self.find_domain_id_by_raw(frame.arg0 as u64)?;
                let resource = self.find_resource_id_by_raw(frame.arg1 as u64)?;
                let kind = match decode_native_contract_kind(frame.arg2) {
                    Some(kind) => kind,
                    None => return Ok(Some(SyscallReturn::err(Errno::Inval))),
                };
                let label = match frame_string(self, caller, frame.arg3, frame.arg4) {
                    Ok(label) => label,
                    Err(result) => return Ok(Some(result)),
                };
                let contract = self.create_contract(caller, domain, resource, kind, label)?;
                Ok(Some(SyscallReturn::ok(contract.raw() as usize)))
            }
            SYS_BIND_PROCESS_CONTRACT => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                self.bind_process_contract(caller, contract)?;
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_LIST_DOMAINS => {
                let ids = self
                    .domain_list()
                    .into_iter()
                    .map(|info| info.id.raw())
                    .collect::<Vec<_>>();
                if let Err(result) =
                    copy_u64_slice_to_user(self, caller, frame.arg0, frame.arg1, &ids)
                {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(ids.len())))
            }
            SYS_INSPECT_DOMAIN => {
                let domain = self.find_domain_id_by_raw(frame.arg0 as u64)?;
                let info = self.domain_info(domain)?;
                let record = NativeDomainRecord {
                    id: info.id.raw(),
                    owner: info.owner.raw(),
                    parent: info.parent.map(|parent| parent.raw()).unwrap_or(0),
                    resource_count: info.resource_count as u64,
                    contract_count: info.contract_count as u64,
                };
                if let Err(result) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_LIST_RESOURCES => {
                let ids = self
                    .resource_list()
                    .into_iter()
                    .map(|info| info.id.raw())
                    .collect::<Vec<_>>();
                if let Err(result) =
                    copy_u64_slice_to_user(self, caller, frame.arg0, frame.arg1, &ids)
                {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(ids.len())))
            }
            SYS_INSPECT_RESOURCE => {
                let resource = self.find_resource_id_by_raw(frame.arg0 as u64)?;
                let info = self.resource_info(resource)?;
                let record = NativeResourceRecord {
                    id: info.id.raw(),
                    domain: info.domain.raw(),
                    creator: info.creator.raw(),
                    holder_contract: info.holder.map(|holder| holder.raw()).unwrap_or(0),
                    kind: encode_native_resource_kind(info.kind) as u32,
                    state: encode_native_resource_state(info.state) as u32,
                    arbitration: encode_native_resource_arbitration_policy(info.arbitration) as u32,
                    governance: encode_native_resource_governance_mode(info.governance) as u32,
                    contract_policy: encode_native_resource_contract_policy(info.contract_policy)
                        as u32,
                    issuer_policy: encode_native_resource_issuer_policy(info.issuer_policy) as u32,
                    waiting_count: info.waiting_count as u64,
                    acquire_count: info.acquire_count,
                    handoff_count: info.handoff_count,
                };
                if let Err(result) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_LIST_RESOURCE_WAITERS => {
                let resource = self.find_resource_id_by_raw(frame.arg0 as u64)?;
                let info = self.resource_info(resource)?;
                let waiters = info
                    .waiters
                    .into_iter()
                    .map(|contract| contract.raw())
                    .collect::<Vec<_>>();
                if let Err(result) =
                    copy_u64_slice_to_user(self, caller, frame.arg1, frame.arg2, &waiters)
                {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(waiters.len())))
            }
            SYS_LIST_CONTRACTS => {
                let ids = self
                    .contract_list()
                    .into_iter()
                    .map(|info| info.id.raw())
                    .collect::<Vec<_>>();
                if let Err(result) =
                    copy_u64_slice_to_user(self, caller, frame.arg0, frame.arg1, &ids)
                {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(ids.len())))
            }
            SYS_INSPECT_CONTRACT => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let info = self.contract_info(contract)?;
                let record = NativeContractRecord {
                    id: info.id.raw(),
                    domain: info.domain.raw(),
                    resource: info.resource.raw(),
                    issuer: info.issuer.raw(),
                    kind: encode_native_contract_kind(info.kind) as u32,
                    state: encode_native_contract_state(info.state) as u32,
                };
                if let Err(result) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_GET_DOMAIN_NAME => {
                let domain = self.find_domain_id_by_raw(frame.arg0 as u64)?;
                let info = self.domain_info(domain)?;
                let copied = copy_string_to_user(self, caller, frame.arg1, frame.arg2, &info.name)?;
                Ok(Some(SyscallReturn::ok(copied)))
            }
            SYS_GET_RESOURCE_NAME => {
                let resource = self.find_resource_id_by_raw(frame.arg0 as u64)?;
                let info = self.resource_info(resource)?;
                let copied = copy_string_to_user(self, caller, frame.arg1, frame.arg2, &info.name)?;
                Ok(Some(SyscallReturn::ok(copied)))
            }
            SYS_GET_CONTRACT_LABEL => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let info = self.contract_info(contract)?;
                let copied =
                    copy_string_to_user(self, caller, frame.arg1, frame.arg2, &info.label)?;
                Ok(Some(SyscallReturn::ok(copied)))
            }
            SYS_SET_CONTRACT_STATE => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let state = match decode_native_contract_state(frame.arg1) {
                    Some(state) => state,
                    None => return Ok(Some(SyscallReturn::err(Errno::Inval))),
                };
                self.transition_contract_state(contract, state)?;
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_INVOKE_CONTRACT => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let invocation_count = self.invoke_contract(contract)?;
                Ok(Some(SyscallReturn::ok(invocation_count as usize)))
            }
            SYS_ACQUIRE_RESOURCE => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let (resource, _) = self.acquire_resource_via_contract(contract)?;
                Ok(Some(SyscallReturn::ok(resource.raw() as usize)))
            }
            SYS_RELEASE_RESOURCE => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let resource = self.release_resource_via_contract(contract)?;
                Ok(Some(SyscallReturn::ok(resource.raw() as usize)))
            }
            SYS_TRANSFER_RESOURCE => {
                let source = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let target = self.find_contract_id_by_raw(frame.arg1 as u64)?;
                let (resource, _) = self.transfer_resource_via_contract(source, target)?;
                Ok(Some(SyscallReturn::ok(resource.raw() as usize)))
            }
            SYS_SET_RESOURCE_POLICY => {
                let resource = self.find_resource_id_by_raw(frame.arg0 as u64)?;
                let policy = match decode_native_resource_arbitration_policy(frame.arg1) {
                    Some(policy) => policy,
                    None => return Ok(Some(SyscallReturn::err(Errno::Inval))),
                };
                self.set_resource_arbitration_policy(resource, policy)?;
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_SET_RESOURCE_GOVERNANCE => {
                let resource = self.find_resource_id_by_raw(frame.arg0 as u64)?;
                let mode = match decode_native_resource_governance_mode(frame.arg1) {
                    Some(mode) => mode,
                    None => return Ok(Some(SyscallReturn::err(Errno::Inval))),
                };
                self.set_resource_governance_mode(resource, mode)?;
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_SET_RESOURCE_CONTRACT_POLICY => {
                let resource = self.find_resource_id_by_raw(frame.arg0 as u64)?;
                let policy = match decode_native_resource_contract_policy(frame.arg1) {
                    Some(policy) => policy,
                    None => return Ok(Some(SyscallReturn::err(Errno::Inval))),
                };
                self.set_resource_contract_policy(resource, policy)?;
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_SET_RESOURCE_ISSUER_POLICY => {
                let resource = self.find_resource_id_by_raw(frame.arg0 as u64)?;
                let policy = match decode_native_resource_issuer_policy(frame.arg1) {
                    Some(policy) => policy,
                    None => return Ok(Some(SyscallReturn::err(Errno::Inval))),
                };
                self.set_resource_issuer_policy(resource, policy)?;
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_SET_RESOURCE_STATE => {
                let resource = self.find_resource_id_by_raw(frame.arg0 as u64)?;
                let state = match decode_native_resource_state(frame.arg1) {
                    Some(state) => state,
                    None => return Ok(Some(SyscallReturn::err(Errno::Inval))),
                };
                self.transition_resource_state(resource, state)?;
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_CLAIM_RESOURCE => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let record = match self.claim_resource_via_contract(contract)? {
                    ResourceClaimResult::Acquired {
                        resource,
                        acquire_count,
                    } => NativeResourceClaimRecord {
                        resource: resource.raw(),
                        holder_contract: contract.raw(),
                        acquire_count,
                        position: 0,
                        queued: 0,
                        reserved: 0,
                    },
                    ResourceClaimResult::Queued {
                        resource,
                        holder,
                        position,
                    } => NativeResourceClaimRecord {
                        resource: resource.raw(),
                        holder_contract: holder.raw(),
                        acquire_count: 0,
                        position: position as u64,
                        queued: 1,
                        reserved: 0,
                    },
                };
                if let Err(result) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_CANCEL_RESOURCE_CLAIM => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let (resource, waiting_count) =
                    self.cancel_resource_claim_via_contract(contract)?;
                let record = NativeResourceCancelRecord {
                    resource: resource.raw(),
                    waiting_count: waiting_count as u64,
                };
                if let Err(result) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(0)))
            }
            SYS_RELEASE_CLAIMED_RESOURCE => {
                let contract = self.find_contract_id_by_raw(frame.arg0 as u64)?;
                let record = match self.release_claimed_resource_via_contract(contract)? {
                    ResourceReleaseResult::Released { resource } => NativeResourceReleaseRecord {
                        resource: resource.raw(),
                        handoff_contract: 0,
                        acquire_count: 0,
                        handoff_count: 0,
                        handed_off: 0,
                        reserved: 0,
                    },
                    ResourceReleaseResult::HandedOff {
                        resource,
                        contract,
                        acquire_count,
                        handoff_count,
                    } => NativeResourceReleaseRecord {
                        resource: resource.raw(),
                        handoff_contract: contract.raw(),
                        acquire_count,
                        handoff_count,
                        handed_off: 1,
                        reserved: 0,
                    },
                };
                if let Err(result) = copy_struct_to_user(self, caller, frame.arg1, &record) {
                    return Ok(Some(result));
                }
                Ok(Some(SyscallReturn::ok(0)))
            }
            _ => Ok(None),
        }
    }
}
