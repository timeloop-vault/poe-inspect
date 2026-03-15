use poe_trade::filter_schema::{FilterIndex, TradeFiltersResponse};

fn load_filters() -> FilterIndex {
    let path = format!(
        "{}/tests/fixtures/trade_filters.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let data = std::fs::read_to_string(&path).expect("failed to read fixture");
    let response: TradeFiltersResponse = serde_json::from_str(&data).expect("failed to parse");
    FilterIndex::from_response(&response)
}

#[test]
fn parses_all_groups() {
    let index = load_filters();
    assert_eq!(index.groups().len(), 12, "expected 12 filter groups");
}

#[test]
fn parses_all_filters() {
    let index = load_filters();
    // From our analysis: 89 total filters across 12 groups
    assert!(
        index.filter_count() >= 85,
        "expected ~89 filters, got {}",
        index.filter_count()
    );
}

#[test]
fn ilvl_is_range() {
    let index = load_filters();
    let f = index
        .filter_def("misc_filters", "ilvl")
        .expect("ilvl filter should exist");
    assert!(f.min_max, "ilvl should be a range filter");
    assert_eq!(f.text.as_deref(), Some("Item Level"));
}

#[test]
fn corrupted_is_option() {
    let index = load_filters();
    let f = index
        .filter_def("misc_filters", "corrupted")
        .expect("corrupted filter should exist");
    assert!(!f.min_max, "corrupted should not be a range filter");
    let opt = f.option.as_ref().expect("corrupted should have options");
    assert_eq!(opt.options.len(), 3, "corrupted should have 3 options (Any/Yes/No)");
}

#[test]
fn rarity_options() {
    let index = load_filters();
    let f = index
        .filter_def("type_filters", "rarity")
        .expect("rarity filter should exist");
    let opt = f.option.as_ref().expect("rarity should have options");
    // Any, Normal, Magic, Rare, Unique, Unique (Foil), Any Non-Unique
    assert!(
        opt.options.len() >= 6,
        "rarity should have >=6 options, got {}",
        opt.options.len()
    );
}

#[test]
fn weapon_filters_exist() {
    let index = load_filters();
    let dps = index.filter_def("weapon_filters", "dps");
    assert!(dps.is_some(), "DPS filter should exist in weapon_filters");
    let pdps = index.filter_def("weapon_filters", "pdps");
    assert!(pdps.is_some(), "pDPS filter should exist");
}

#[test]
fn armour_filters_exist() {
    let index = load_filters();
    let ar = index.filter_def("armour_filters", "ar");
    assert!(ar.is_some(), "Armour filter should exist");
    let es = index.filter_def("armour_filters", "es");
    assert!(es.is_some(), "Energy Shield filter should exist");
}

#[test]
fn fractured_filter_exists() {
    let index = load_filters();
    let f = index
        .filter_def("misc_filters", "fractured_item")
        .expect("fractured_item should exist");
    let opt = f.option.as_ref().expect("should have options");
    assert_eq!(opt.options.len(), 3);
}

#[test]
fn foulborn_filter_exists() {
    let index = load_filters();
    let f = index.filter_def("misc_filters", "mutated");
    assert!(f.is_some(), "Foulborn (mutated) filter should exist");
}
