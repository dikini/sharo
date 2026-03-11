use std::path::PathBuf;

use proptest::prelude::*;
use proptest::string::string_regex;
use sharo_core::skills::derive_skill_id;

fn valid_segment() -> impl Strategy<Value = String> {
    string_regex("[a-z0-9_-]{1,8}").expect("valid regex")
}

proptest! {
    #[test]
    fn skill_id_derivation_is_stable_for_valid_relative_paths(segments in prop::collection::vec(valid_segment(), 1..=4)) {
        let root = PathBuf::from("/tmp/project/.agents/skills");
        let skill_dir = segments.iter().fold(root.clone(), |path, segment| path.join(segment));
        let first = derive_skill_id(&root, &skill_dir).expect("skill id");
        let second = derive_skill_id(&root, &skill_dir).expect("skill id");

        prop_assert_eq!(&first, &second);
        prop_assert!(!first.is_empty());
        prop_assert!(!first.contains('\\'));
    }
}
