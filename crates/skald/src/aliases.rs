pub fn resolve_table_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "name" => "firstname".to_string(),
        "pro" => "pron".to_string(),
        "with" => "preposition".to_string(),
        _ => lower,
    }
}

pub fn resolve_arg_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "pl" => "plural".to_string(),
        "dposs" => "poss".to_string(),
        _ => lower,
    }
}
