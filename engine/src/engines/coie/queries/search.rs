use crate::engines::coie::model::GraphEntity;

pub fn by_name<'a>(entities: &'a [GraphEntity], name: &str) -> Vec<&'a GraphEntity> {
    entities
        .iter()
        .filter(|entity| entity.name == name)
        .collect()
}
