//! Creates isolated directories for native TUI plan integration fixtures.

use scriberig_core::EntityId;

/// Creates a uniquely named agent-owned test directory.
///
/// - `label`: Short representative name for the temporary fixture.
///
/// # Returns
/// A path below `playground/agentic-work` with the required timestamp prefix.
pub(super) fn agentic_fixture_root(label: &str) -> std::path::PathBuf {
    assert!(
        label.len() <= 40,
        "fixture label must be at most 40 characters"
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after Unix epoch");
    let seconds = i64::try_from(now.as_secs()).expect("timestamp fits i64");
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    }
    .div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        .div_euclid(365);
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2).div_euclid(153);
    let day = day_of_year - (153 * month_part + 2).div_euclid(5) + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    let second = day_seconds % 60;
    let timestamp = format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}");
    let identifier = EntityId::new().to_string();
    let suffix = &identifier[4..12];
    let directory_name = format!("{timestamp} {label}-{suffix}");
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../playground/agentic-work")
        .join(directory_name);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}
