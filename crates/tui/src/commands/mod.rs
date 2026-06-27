use cleanr_agent::{ActionRequest, CommandInfo, command_palette};
use cleanr_i18n::I18n;

pub(crate) fn filtered_palette_commands(
    has_scan_results: bool,
    input: &str,
    i18n: &I18n,
) -> Vec<CommandInfo> {
    let filter = input.strip_prefix('/').unwrap_or("").trim().to_lowercase();
    command_palette(has_scan_results)
        .into_iter()
        .filter(|command| {
            filter.is_empty()
                || command.name.to_lowercase().contains(&filter)
                || i18n
                    .t(command.description_key)
                    .to_lowercase()
                    .contains(&filter)
        })
        .collect()
}

pub(crate) fn command_name_for_status(action: &ActionRequest) -> &'static str {
    match action {
        ActionRequest::Scan(_) => "/scan",
        ActionRequest::Review => "/review",
        ActionRequest::Plan => "/plan",
        ActionRequest::Clean { .. } => "/clean",
        ActionRequest::Restore => "/restore",
        ActionRequest::Rules => "/rules",
        ActionRequest::Plugins => "/plugins",
        ActionRequest::Languages => "/languages",
        ActionRequest::Tasks => "/tasks",
        ActionRequest::Usage(_) => "/usage",
        ActionRequest::ExportPlan(_) => "/export-plan",
        ActionRequest::Help => "/help",
        ActionRequest::Quit => "/quit",
    }
}

pub(crate) fn palette_command_invocation(name: &str) -> String {
    name.split_whitespace()
        .filter(|part| !part.starts_with('['))
        .collect::<Vec<_>>()
        .join(" ")
}
