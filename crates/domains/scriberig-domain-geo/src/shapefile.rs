//! Classifies physical Shapefile members and their case-folded group identities.

#[path = "shapefile/reader.rs"]
mod reader;
pub use reader::{ShapefileError, ShapefileLayer, ShapefileReader};

/// Describes a Shapefile sidecar path recognized by the geospatial adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapefileMemberPath {
    /// Exact filename stem used to avoid grouping case-distinct datasets.
    pub stem: String,
    /// Lowercase stem used to detect collisions on case-folding filesystems.
    pub normalized_stem: String,
}

/// Classifies one project-relative Shapefile member path.
///
/// # Parameters
/// - `path`: Project-relative POSIX path from the persisted scan.
///
/// # Returns
/// The exact and case-normalized dataset stems for recognized sidecar files.
#[must_use]
pub fn shapefile_member_path(path: &str) -> Option<ShapefileMemberPath> {
    let name = path.rsplit('/').next()?;
    let (stem, extension) = name.rsplit_once('.')?;
    if stem.is_empty()
        || !matches!(
            extension.to_ascii_lowercase().as_str(),
            "shp" | "shx" | "dbf" | "prj" | "cpg" | "qix"
        )
    {
        return None;
    }
    Some(ShapefileMemberPath {
        stem: stem.to_owned(),
        normalized_stem: stem.to_lowercase(),
    })
}

#[cfg(test)]
mod tests {
    use super::shapefile_member_path;

    /// Recognizes standard sidecar extensions from the filename component.
    #[test]
    fn classifies_shapefile_sidecars_by_filename() {
        assert_eq!(
            shapefile_member_path("gis/Roads.SHP").map(|member| member.stem),
            Some("Roads".to_owned())
        );
        assert_eq!(
            shapefile_member_path("gis/roads.dbf").map(|member| member.normalized_stem),
            Some("roads".to_owned())
        );
        assert!(shapefile_member_path("gis/roads.txt").is_none());
        assert!(shapefile_member_path("gis/.shp").is_none());
    }
}
