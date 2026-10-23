use sol_adaptor_moose::hit::{
    append_top_level_block, get_parameter, remove_block, remove_parameter, replace_block,
    upsert_parameter, HitError, MooseInput,
};

const LF: &str = "# α comment\n[Variables]\n  [./u] # child\n    type = FVReal\n  [../]\n[]\n[Executioner]\n  type = Transient\n  dt = 1e-4 # keep\n[]\n";

#[test]
fn parses_nested_close_forms_and_direct_children() {
    let doc = MooseInput::parse(LF).unwrap();
    assert_eq!(doc.unique("Variables/u").unwrap().path, "Variables/u");
    assert_eq!(doc.direct_children("Variables"), vec!["Variables/u"]);
}

#[test]
fn malformed_and_ambiguous_input_fails_closed() {
    assert!(matches!(
        MooseInput::parse("[]\n"),
        Err(HitError::UnmatchedClose { .. })
    ));
    assert!(matches!(
        MooseInput::parse("[A]\n"),
        Err(HitError::UnclosedBlock { .. })
    ));
    let dup = "[A]\n[]\n[A]\n[]\n";
    assert!(matches!(
        MooseInput::parse(dup).unwrap().unique("A"),
        Err(HitError::AmbiguousBlock { count: 2, .. })
    ));
}

#[test]
fn parameter_edits_are_local_and_preserve_comments() {
    let changed = upsert_parameter(LF, "Executioner", "dt", "2e-4").unwrap();
    assert!(changed.contains("dt = 2e-4 # keep"));
    assert!(changed.starts_with("# α comment\n[Variables]"));
    assert_eq!(
        get_parameter(&changed, "Executioner", "dt")
            .unwrap()
            .as_deref(),
        Some("2e-4")
    );
    let inserted = upsert_parameter(&changed, "Executioner", "end_time", "1e-3").unwrap();
    assert!(inserted.contains("  end_time = 1e-3\n[]"));
    let removed = remove_parameter(&inserted, "Executioner", "end_time").unwrap();
    assert_eq!(removed, changed);
}

#[test]
fn duplicate_parameter_is_rejected() {
    let dup = LF.replace("  dt = 1e-4 # keep\n", "  dt = 1e-4\n  dt = 2e-4\n");
    assert!(matches!(
        upsert_parameter(&dup, "Executioner", "dt", "3e-4"),
        Err(HitError::AmbiguousParameter { count: 2, .. })
    ));
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

#[test]
fn parent_parameters_do_not_leak_into_nested_child() {
    let input = "[A]\n  value = parent\n  [./B]\n    value = child\n  [../]\n[]\n";
    assert_eq!(
        get_parameter(input, "A", "value").unwrap().as_deref(),
        Some("parent")
    );
    let updated = upsert_parameter(input, "A", "value", "updated").unwrap();
    assert!(updated.contains("  value = updated\n"));
    assert!(updated.contains("    value = child\n"));
    let removed = remove_parameter(input, "A", "value").unwrap();
    assert!(!removed.contains("  value = parent\n"));
    assert!(removed.contains("    value = child\n"));

    let child_only = "[A]\n  [./B]\n    value = child\n  [../]\n[]\n";
    assert_eq!(get_parameter(child_only, "A", "value").unwrap(), None);
    let inserted = upsert_parameter(child_only, "A", "value", "parent").unwrap();
    assert!(inserted.contains("    value = child\n"));
    assert!(inserted.contains("  value = parent\n"));
}

#[test]
fn commented_close_line_is_accepted_for_local_parameter_insertion() {
    let text = "# α retained\n[A]\n  [./B] # child\n  [../] # child end\n[] # parent end\n";
    let inserted = upsert_parameter(text, "A", "value", "42").unwrap();
    assert_eq!(
        inserted,
        "# α retained\n[A]\n  [./B] # child\n  [../] # child end\n  value = 42\n[] # parent end\n"
    );
}

#[test]
fn append_and_replace_reject_payload_without_a_complete_block() {
    assert!(matches!(
        append_top_level_block("[A]\n[]\n", "not a block"),
        Err(HitError::InvalidMutation { .. })
    ));
    assert!(matches!(
        replace_block("[A]\n[]\n", "A", "not a block"),
        Err(HitError::InvalidMutation { .. })
    ));
    assert!(matches!(
        append_top_level_block("[A]\n[]\n", "[B]\n[]\n[C]\n[]"),
        Err(HitError::InvalidMutation { .. })
    ));
    assert!(matches!(
        replace_block("[A]\n[]\n", "A", "[B]\n[]\n[C]\n[]"),
        Err(HitError::InvalidMutation { .. })
    ));
    assert!(matches!(
        replace_block("[A]\n[]\n", "A", "[B]\n"),
        Err(HitError::UnclosedBlock { .. })
    ));
}

#[test]
fn ambiguous_child_assignments_do_not_change_parent_parameter_ownership() {
    let text = "[A]\n  [./B]\n    p = child\n  [../]\n[]\n";
    assert_eq!(get_parameter(text, "A", "p").unwrap(), None);
    let inserted = upsert_parameter(text, "A", "p", "parent").unwrap();
    assert!(inserted.contains("    p = child\n"));
    assert!(inserted.contains("  p = parent\n[]"));
}
