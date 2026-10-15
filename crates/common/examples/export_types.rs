use packy_common::models::*;
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/shared/generated");
    Pack::export_all_to(&output)?;
    PackInput::export_all_to(&output)?;
    Artifact::export_all_to(&output)?;
    ArtifactInput::export_all_to(&output)?;
    Release::export_all_to(&output)?;
    ReleaseItem::export_all_to(&output)?;
    CreateDraftRequest::export_all_to(&output)?;
    DraftItemInput::export_all_to(&output)?;
    UpdateDraftRequest::export_all_to(&output)?;
    RevisionRequest::export_all_to(&output)?;
    UpdateAccountRequest::export_all_to(&output)?;
    Role::export_all_to(&output)?;
    Permission::export_all_to(&output)?;
    UpdateRolePermissionsRequest::export_all_to(&output)?;
    HostedFile::export_all_to(&output)?;
    Page::<String>::export_all_to(&output)?;
    DownloadSource::export_all_to(&output)?;
    TargetFile::export_all_to(&output)?;
    Operation::export_all_to(&output)?;
    UpdatePlanRequest::export_all_to(&output)?;
    UpdatePlan::export_all_to(&output)?;
    ApiError::export_all_to(&output)?;
    Credentials::export_all_to(&output)?;
    LoginResponse::export_all_to(&output)?;
    Account::export_all_to(&output)?;
    SystemSettings::export_all_to(&output)?;
    SourceDictionaryItem::export_all_to(&output)?;
    ModRecord::export_all_to(&output)?;
    ModInput::export_all_to(&output)?;
    LocalConfig::export_all_to(&output)?;
    ClientPreferences::export_all_to(&output)?;
    LocalState::export_all_to(&output)?;
    TaskRecord::export_all_to(&output)?;
    TaskStatus::export_all_to(&output)?;
    Health::export_all_to(&output)?;
    ProbeInfo::export_all_to(&output)?;
    ClientSnapshot::export_all_to(&output)?;
    Ok(())
}
