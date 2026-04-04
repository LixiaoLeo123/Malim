use std::collections::HashMap;
use super::{NewsScraper, ria::RiaScraper, kp::KpScraper, yna::YnaScraper, kbs::KbsScraper, tmt::TmtScraper};

pub fn get_scrapers_by_language() -> HashMap<String, Vec<Box<dyn NewsScraper>>> {
    let mut map: HashMap<String, Vec<Box<dyn NewsScraper>>> = HashMap::new();

    map.insert(
        "ru".to_string(),
        vec![
            Box::new(RiaScraper::new()),
            Box::new(KpScraper::new()),
            Box::new(TmtScraper::new()),
        ],
    );

    map.insert(
        "kr".to_string(),
        vec![
            Box::new(YnaScraper::new()),
            Box::new(KbsScraper::new()),
        ],
    );

    map
}
