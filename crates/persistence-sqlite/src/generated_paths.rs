pub fn include_str_path_literal(path: &str) -> String {
    let mut hashes = String::new();

    while path.contains(&format!("\"{hashes}")) {
        hashes.push('#');
    }

    format!("r{hashes}\"{path}\"{hashes}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_paths_use_raw_string_literal() {
        let path = r"D:\a\peekoo-ai\crates\persistence-sqlite/migrations/202602150001_init.sql";

        assert_eq!(
            include_str_path_literal(path),
            "r\"D:\\a\\peekoo-ai\\crates\\persistence-sqlite/migrations/202602150001_init.sql\""
        );
    }

    #[test]
    fn unix_paths_still_use_raw_string_literal() {
        let path = "/workspace/crates/persistence-sqlite/migrations/202602150001_init.sql";

        assert_eq!(
            include_str_path_literal(path),
            "r\"/workspace/crates/persistence-sqlite/migrations/202602150001_init.sql\""
        );
    }
}
