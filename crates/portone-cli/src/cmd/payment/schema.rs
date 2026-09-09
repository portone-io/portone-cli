use std::sync::LazyLock;

use clap::{ValueEnum, builder::PossibleValue};
use portone_schema_macros::schema_enum;

schema_enum!(pub PaymentStatus);
schema_enum!(pub PaymentMethodType);
schema_enum!(pub PgProvider);
schema_enum!(pub Currency, cli_case = "preserve");
schema_enum!(pub PaymentTimestampType);
schema_enum!(pub PaymentSortBy);
schema_enum!(pub SortOrder);
schema_enum!(pub PaymentTextSearchField);
schema_enum!(pub PortOneVersion);

/// `all` is a CLI filter control, not an API version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionFilter {
    Version(PortOneVersion),
    All,
}

impl VersionFilter {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Version(version) => version.as_api_str(),
            Self::All => "ALL",
        }
    }
}

impl ValueEnum for VersionFilter {
    fn value_variants<'a>() -> &'a [Self] {
        static VALUES: LazyLock<Vec<VersionFilter>> = LazyLock::new(|| {
            PortOneVersion::value_variants()
                .iter()
                .copied()
                .map(VersionFilter::Version)
                .chain([VersionFilter::All])
                .collect()
        });
        &VALUES
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Version(version) => version.to_possible_value(),
            Self::All => Some(PossibleValue::new("all")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_filter_delegates_every_schema_version() {
        for version in PortOneVersion::value_variants() {
            let value = version.to_possible_value().unwrap();
            let filter = VersionFilter::from_str(value.get_name(), false).unwrap();
            assert_eq!(filter, VersionFilter::Version(*version));
            assert_eq!(filter.as_api_str(), version.as_api_str());
        }
        assert_eq!(
            VersionFilter::from_str("all", false).unwrap(),
            VersionFilter::All
        );
        assert_eq!(
            VersionFilter::value_variants().len(),
            PortOneVersion::value_variants().len() + 1
        );
    }
}
