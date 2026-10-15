use crate::engines::coie::model::{EntityKind, GraphEntity};

pub fn symbols(entities: &[GraphEntity]) -> impl Iterator<Item = &GraphEntity> {
    entities
        .iter()
        .filter(|entity| entity.kind == EntityKind::Symbol)
}
