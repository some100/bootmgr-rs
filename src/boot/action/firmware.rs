use uefi::{
    Status, cstr16,
    runtime::{self, ResetType, VariableAttributes, VariableVendor, get_variable, set_variable},
};

pub fn reset_to_firmware() -> uefi::Result<()> {
    let mut buf = [0; size_of::<u64>()];
    match get_variable(
        cstr16!("OsIndications"),
        &VariableVendor::GLOBAL_VARIABLE,
        &mut buf,
    ) {
        Err(e) if e.status() != Status::NOT_FOUND => return Err(e.to_err_without_payload()), // if its not found, create one
        _ => (),
    }
    let mut osind = u64::from_le_bytes(buf); // this is fine since buf is 0 initialized
    osind |= 1; // EFI_OS_INDICATIONS_BOOT_TO_FW_UI
    buf = u64::to_le_bytes(osind);
    set_variable(
        cstr16!("OsIndications"),
        &VariableVendor::GLOBAL_VARIABLE,
        VariableAttributes::NON_VOLATILE
            .union(VariableAttributes::BOOTSERVICE_ACCESS)
            .union(VariableAttributes::RUNTIME_ACCESS),
        &mut buf,
    )?;
    runtime::reset(ResetType::WARM, Status::SUCCESS, None);
}
