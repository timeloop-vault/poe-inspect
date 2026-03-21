use crate::game_data::GameDataState;
use crate::ProfileState;
use crate::commands::inspect::ItemPayload;

/// Built-in default profile — compiled into the binary, can never be deleted.
const DEFAULT_PROFILE_JSON: &str = include_str!("../../data/profiles/generic.json");

/// Parse the built-in default profile.
pub(crate) fn default_profile() -> Option<poe_eval::Profile> {
    match serde_json::from_str(DEFAULT_PROFILE_JSON) {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("Failed to parse built-in default profile: {e}");
            None
        }
    }
}

/// Parse and evaluate item text from clipboard.
/// Returns item display data + evaluation results, or an error string.
#[tauri::command]
pub(crate) fn evaluate_item(
    item_text: String,
    state: tauri::State<'_, GameDataState>,
) -> Result<ItemPayload, String> {
    let gd = &state.0;

    // Pass 1: structural parse
    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;

    // Pass 2: resolve against game data
    let resolved = poe_item::resolve(&raw, gd);

    // Evaluate (no profile for direct command calls)
    let evaluation = poe_eval::evaluate_item(&resolved, gd, None, &[]);
    Ok(ItemPayload {
        item: resolved,
        eval: evaluation,
        raw_text: item_text,
    })
}

/// Set primary + watching profiles from the frontend.
/// primaryJson: poe-eval Profile JSON (empty = built-in default).
/// watchingJson: JSON array of {name, color, profile} objects.
#[tauri::command]
pub(crate) fn set_active_profile(
    primary_json: String,
    watching_json: String,
    state: tauri::State<'_, ProfileState>,
) {
    use poe_eval::{Profile, WatchingProfileInput};

    // "none" = no primary (show overlay without scoring)
    // "" = use built-in default profile
    // JSON = custom profile
    let primary = if primary_json == "none" {
        None
    } else if primary_json.is_empty() {
        default_profile()
    } else {
        match serde_json::from_str::<Profile>(&primary_json) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("Failed to parse primary profile: {e}");
                default_profile()
            }
        }
    };

    let watching: Vec<WatchingProfileInput> =
        serde_json::from_str(&watching_json).unwrap_or_default();

    eprintln!(
        "[profiles] Primary: {}, Watching: {}",
        if primary.is_some() { "set" } else { "none" },
        watching.len()
    );

    let mut ps = state.0.lock().unwrap();
    ps.primary = primary;
    ps.watching = watching;
}

/// Return the built-in default profile so the frontend can display or customize it.
#[tauri::command]
pub(crate) fn get_default_profile() -> Option<String> {
    default_profile().map(|p| serde_json::to_string(&p).unwrap_or_default())
}

/// Return the predicate schema so the frontend can build profile editors dynamically.
#[tauri::command]
pub(crate) fn get_predicate_schema() -> Vec<poe_eval::PredicateSchema> {
    poe_eval::predicate_schema()
}

/// Return suggestion values for a given data source.
/// Used by the profile editor for autocomplete on text fields.
#[tauri::command]
pub(crate) fn get_suggestions(source: String, state: tauri::State<'_, GameDataState>) -> Vec<String> {
    let gd = &state.0;
    match source.as_str() {
        "item_classes" => {
            let mut names: Vec<String> = gd.item_classes.iter().map(|c| c.name.clone()).collect();
            names.sort();
            names
        }
        "base_types" => {
            let mut names: Vec<String> =
                gd.base_item_types.iter().map(|b| b.name.clone()).collect();
            names.sort();
            names
        }
        "mod_names" => {
            let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for m in &gd.mods {
                if !m.name.is_empty() {
                    names.insert(m.name.clone());
                }
            }
            names.into_iter().collect()
        }
        "stat_texts" => gd.all_stat_templates(),
        "stat_ids" => {
            let mut ids: Vec<String> = gd.stats.iter().map(|s| s.id.clone()).collect();
            ids.sort();
            ids
        }
        _ => vec![],
    }
}

/// A map mod template with its stat IDs (for the map danger settings page).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MapModTemplate {
    template: String,
    stat_ids: Vec<String>,
}

/// Return all map/area mod templates for the map danger settings page.
#[tauri::command]
pub(crate) fn get_map_mod_templates(state: tauri::State<'_, GameDataState>) -> Vec<MapModTemplate> {
    let mut templates: Vec<MapModTemplate> = state
        .0
        .map_mod_templates()
        .into_iter()
        .filter(|(template, _)| template.chars().any(|c| c.is_alphabetic()))
        .map(|(template, stat_ids)| MapModTemplate {
            template: template.to_string(),
            stat_ids: stat_ids.to_vec(),
        })
        .collect();
    templates.sort_by(|a, b| a.template.cmp(&b.template));
    templates
}

/// Return enriched stat suggestions matching a text query.
///
/// Returns both single-stat suggestions and hybrid mod combos that include
/// the matching stat. Used by the stat picker to show hybrid options.
#[tauri::command]
pub(crate) fn get_stat_suggestions(
    query: String,
    state: tauri::State<'_, GameDataState>,
) -> Vec<poe_data::StatSuggestion> {
    state.0.stat_suggestions_for_query(&query)
}

/// Resolve stat IDs to their human-readable template text.
/// Returns a map of stat_id -> template (first match). Unknown IDs are omitted.
#[tauri::command]
pub(crate) fn resolve_stat_templates(
    stat_ids: Vec<String>,
    state: tauri::State<'_, GameDataState>,
) -> std::collections::HashMap<String, String> {
    let mut result = std::collections::HashMap::new();
    for stat_id in &stat_ids {
        if let Some(templates) = state.0.templates_for_stat(stat_id) {
            if let Some(first) = templates.first() {
                result.insert(stat_id.clone(), first.clone());
            }
        }
    }
    result
}
