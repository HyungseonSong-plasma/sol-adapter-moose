use sol_adaptor_moose::hit::{append_top_level_block, get_parameter, remove_block, remove_parameter, replace_block, upsert_parameter, HitError, MooseInput};

const LF: &str = "# α comment\n[Variables]\n  [./u] # child\n    type = FVReal\n  [../]\n[]\n[Executioner]\n  type = Transient\n  dt = 1e-4 # keep\n[]\n";

#[test]
fn parses_nested_close_forms_and_direct_children() {
    let doc = MooseInput::parse(LF).unwrap();
    assert_eq!(doc.unique("Variables/u").unwrap().path, "Variables/u");
    assert_eq!(doc.direct_children("Variables"), vec!["Variables/u"]);
}

#[test]
fn malformed_and_ambiguous_input_fails_closed() {
    assert!(matches!(MooseInput::parse("[]\n"), Err(HitError::UnmatchedClose { .. })));
    assert!(matches!(MooseInput::parse("[A]\n"), Err(HitError::UnclosedBlock { .. })));
    let dup = "[A]\n[]\n[A]\n[]\n";
    assert!(matches!(MooseInput::parse(dup).unwrap().unique("A"), Err(HitError::AmbiguousBlock { count: 2, .. })));
}

#[test]
fn parameter_edits_are_local_and_preserve_comments() {
    let changed = upsert_parameter(LF, "Executioner", "dt", "2e-4").unwrap();
    assert!(changed.contains("dt = 2e-4 # keep"));
    assert!(changed.starts_with("# α comment\n[Variables]"));
    assert_eq!(get_parameter(&changed, "Executioner", "dt").unwrap().as_deref(), Some("2e-4"));
    let inserted = upsert_parameter(&changed, "Executioner", "end_time", "1e-3").unwrap();
    assert!(inserted.contains("  end_time = 1e-3\n[]"));
    let removed = remove_parameter(&inserted, "Executioner", "end_time").unwrap();
    assert_eq!(removed, changed);
}

#[test]
fn duplicate_parameter_is_rejected() {
    let dup = LF.replace("  dt = 1e-4 # keep\n", "  dt = 1e-4\n  dt = 2e-4\n");
    assert!(matches!(upsert_parameter(&dup, "Executioner", "dt", "3e-4"), Err(HitError::AmbiguousParameter { count: 2, .. })));
}

#[test]
fn block_edits_preserve_unrelated_text() {
    let removed = remove_block(LF, "Variables/u").unwrap();
    assert!(removed.starts_with("# α comment\n[Variables]\n"));
    assert!(removed.contains("[Executioner]\n  type = Transient"));
    let replacement = "  [v]\n    type = FVReal\n  []";
    let replaced = replace_block(LF, "Variables/u", replacement).unwrap();
    assert!(replaced.contains("  [v]\n    type = FVReal\n  []\n[]"));
}

#[test]
fn crlf_is_parsed_without_global_normalization() {
    let text = "[A]\r\n  value = old # c\r\n[]\r\n";
    let changed = upsert_parameter(text, "A", "value", "new").unwrap();
    assert_eq!(changed, "[A]\r\n  value = new # c\r\n[]\r\n");
}

#[test]
fn insert_and_append_validate_result() {
    let doc = MooseInput::parse("[A]\n[]\n").unwrap();
    let child = doc.insert_before_close("A", "  [B]\n  []").unwrap();
    assert!(MooseInput::parse(&child).unwrap().unique("A/B").is_ok());
    let appended = append_top_level_block("[A]\n[]", "[B]\n[]").unwrap();
    assert!(MooseInput::parse(&appended).unwrap().unique("B").is_ok());
}

#[test]
fn nested_parent_and_child_cannot_be_removed_as_one_overlapping_batch_by_accident() {
    let text = "[A]\n  [B]\n  []\n[]\n[C]\n[]\n";
    let child_only = remove_block(text, "A/B").unwrap();
    assert!(child_only.contains("[A]\n[]\n"));
    assert!(child_only.contains("[C]\n[]\n"));
}
