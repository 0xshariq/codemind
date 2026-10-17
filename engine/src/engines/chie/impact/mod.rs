pub mod aggregation;
pub mod traversal;

#[cfg(test)]
mod integration_tests {
    use super::{aggregation::deduplicate, traversal::traverse};
    use crate::engines::chie::model::{ImpactClassification, ImpactPath};

    #[test]
    fn impact_module_composes_traversal_and_aggregation_without_losing_distinct_paths() {
        let path = ImpactPath {
            entity_id: "caller".into(),
            depth: 1,
            classification: ImpactClassification::Direct,
            relationships: Vec::new(),
        };
        assert_eq!(deduplicate(vec![path.clone(), path]).len(), 1);
        assert!(traverse(&[], "target", 3).is_empty());
    }
}
