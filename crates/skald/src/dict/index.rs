use super::types::Entry;
use std::collections::HashMap;

pub fn build_table_index(entries: &[Entry]) -> (HashMap<String, Vec<usize>>, bool) {
    let mut by_class: HashMap<String, Vec<usize>> = HashMap::new();
    let mut has_nsfw = false;

    let add = |cls: &str, i: usize, by_class: &mut HashMap<String, Vec<usize>>| {
        let bucket = by_class.entry(cls.to_string()).or_default();
        if bucket.last().copied() != Some(i) {
            bucket.push(i);
        }
    };

    for (i, entry) in entries.iter().enumerate() {
        let mut male_q = false;
        let mut female_q = false;
        for c in &entry.classes {
            add(c, i, &mut by_class);
            match c.as_str() {
                "male?" => {
                    add("male", i, &mut by_class);
                    male_q = true;
                }
                "female?" => {
                    add("female", i, &mut by_class);
                    female_q = true;
                }
                "nsfw" => has_nsfw = true,
                _ => {}
            }
        }
        if male_q && female_q {
            add("neutral", i, &mut by_class);
        }
    }

    (by_class, has_nsfw)
}
