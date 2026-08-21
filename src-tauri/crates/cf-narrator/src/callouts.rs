//! Callout prettifier (issue #2): raw `last_place_name` → coach English.
//! One canonical implementation — narrator templates, habit cards, the RBR
//! rail, and (V1.4) the replay map all read from here. Total function:
//! unknown callouts get CamelCase splitting, never an error.

/// Curated names where splitting reads wrong. Keep sorted; add as real
/// demos surface new raw values (record them in the PR that adds them).
const OVERRIDES: &[(&str, &str)] = &[
    ("BombsiteA", "A site"),
    ("BombsiteB", "B site"),
    ("CTSpawn", "CT spawn"),
    ("TSpawn", "T spawn"),
    ("TRamp", "T ramp"),
];

/// Prettify a raw callout: curated override, else split CamelCase runs
/// ("PalaceInterior" → "Palace Interior"). Consecutive capitals stay
/// together as an acronym ("CT"); a single capital word stands alone
/// ("LongA" → "Long A").
pub fn callout_name(raw: &str) -> String {
    if let Some((_, pretty)) = OVERRIDES.iter().find(|(k, _)| *k == raw) {
        return (*pretty).to_string();
    }
    let mut out = String::with_capacity(raw.len() + 4);
    let chars: Vec<char> = raw.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0
            && c.is_ascii_uppercase()
            && (chars[i - 1].is_ascii_lowercase()
                || (i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase()))
        {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::callout_name;

    #[test]
    fn curated_overrides() {
        assert_eq!(callout_name("BombsiteA"), "A site");
        assert_eq!(callout_name("BombsiteB"), "B site");
        assert_eq!(callout_name("TSpawn"), "T spawn");
        assert_eq!(callout_name("CTSpawn"), "CT spawn");
        assert_eq!(callout_name("TRamp"), "T ramp");
    }

    #[test]
    fn camelcase_splitting() {
        assert_eq!(callout_name("SideHall"), "Side Hall");
        assert_eq!(callout_name("PalaceInterior"), "Palace Interior");
        assert_eq!(callout_name("BackAlley"), "Back Alley");
        assert_eq!(callout_name("Underpass"), "Underpass");
        assert_eq!(callout_name("Catwalk"), "Catwalk");
    }

    #[test]
    fn acronym_runs_stay_together() {
        assert_eq!(callout_name("CTSpawn"), "CT spawn"); // curated, but…
        assert_eq!(callout_name("LongA"), "Long A"); // trailing single capital
        assert_eq!(callout_name("APlatform"), "A Platform");
    }

    #[test]
    fn degenerate_inputs_pass_through() {
        assert_eq!(callout_name(""), "");
        assert_eq!(callout_name("lower"), "lower");
    }
}
