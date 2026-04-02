use std::collections::HashMap;
use super::{NewsScraper, ria::RiaScraper};

pub fn get_scrapers_by_language() -> HashMap<String, Vec<Box<dyn NewsScraper>>> {
    let mut map: HashMap<String, Vec<Box<dyn NewsScraper>>> = HashMap::new();

    map.insert(
        "ru".to_string(),
        vec![
            Box::new(RiaScraper::new()),
        ],
    );

    map.insert(
        "kr".to_string(),
        vec![
        ],
    );

    map
}
