//! The declared Annex IV inputs round-trip, and the two checks that catch a
//! declaration the annex cannot grade.

use super::index_scope_exclusion::IndexScopeExclusion;
use super::priority_part_scores::{MAX_SCORE, MIN_SCORE, PriorityPartScores};
use super::repairability_index::RepairabilityIndexDeclaration;

fn parts(folding: Option<u8>) -> PriorityPartScores {
    PriorityPartScores {
        battery: 5,
        display_assembly: 4,
        back_cover: 5,
        front_camera: 3,
        rear_camera: 3,
        charging_port: 4,
        mechanical_button: 5,
        microphone: 4,
        speaker: 4,
        folding_mechanism: folding,
    }
}

fn declaration(folding: Option<u8>) -> RepairabilityIndexDeclaration {
    RepairabilityIndexDeclaration {
        disassembly_depth: parts(folding),
        fasteners: parts(folding),
        tools: parts(folding),
        spare_parts: 5,
        software_updates: 5,
        repair_information: 4,
    }
}

#[test]
fn the_ten_priority_parts_use_annex_iv_wire_names() {
    let json = serde_json::to_value(parts(Some(3))).expect("serialises");
    // Annex IV point 5's ten abbreviations, in its own order: BAT, DA, BC, FFC,
    // RFC, EC, BUT, MIC, SPK, FM.
    for key in [
        "battery",
        "displayAssembly",
        "backCover",
        "frontCamera",
        "rearCamera",
        "chargingPort",
        "mechanicalButton",
        "microphone",
        "speaker",
        "foldingMechanism",
    ] {
        assert!(json.get(key).is_some(), "missing {key} in {json}");
    }
    assert_eq!(
        json.as_object().expect("object").len(),
        10,
        "the annex names ten priority parts and no more"
    );
}

#[test]
fn a_non_foldable_declaration_omits_the_hinge_entirely() {
    let json = serde_json::to_value(parts(None)).expect("serialises");
    assert!(
        json.get("foldingMechanism").is_none(),
        "a key present but null would read as a declared score of nothing"
    );
    let back: PriorityPartScores = serde_json::from_value(json).expect("round-trips");
    assert!(!back.is_foldable());
}

#[test]
fn the_six_parameters_round_trip() {
    let declared = declaration(None);
    let json = serde_json::to_value(&declared).expect("serialises");
    for key in [
        "disassemblyDepth",
        "fasteners",
        "tools",
        "spareParts",
        "softwareUpdates",
        "repairInformation",
    ] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    let back: RepairabilityIndexDeclaration = serde_json::from_value(json).expect("round-trips");
    assert_eq!(back, declared);
}

#[test]
fn a_declaration_that_folds_under_one_parameter_only_is_refused() {
    // Annex IV picks between two weight sets on the hinge's presence, so a
    // declaration disagreeing with itself does not describe a product the annex
    // can grade. The failure is otherwise silent — the index would be computed
    // against whichever weight set the first parameter suggested.
    assert!(declaration(Some(3)).foldable_is_consistent());
    assert!(declaration(None).foldable_is_consistent());

    let mut mixed = declaration(Some(3));
    mixed.tools.folding_mechanism = None;
    assert!(!mixed.foldable_is_consistent());
}

#[test]
fn every_score_must_sit_inside_the_annexs_one_to_five() {
    assert!(declaration(Some(MAX_SCORE)).scores_are_in_range());
    assert!(declaration(Some(MIN_SCORE)).scores_are_in_range());

    let mut too_high = declaration(None);
    too_high.disassembly_depth.battery = MAX_SCORE + 1;
    assert!(!too_high.scores_are_in_range(), "6 is not a point level");

    let mut zero = declaration(None);
    zero.spare_parts = 0;
    assert!(!zero.scores_are_in_range(), "0 is not a point level");

    let mut hinge = declaration(Some(9));
    assert!(!hinge.scores_are_in_range(), "the hinge is scored 1-5 too");
    hinge.disassembly_depth.folding_mechanism = Some(MAX_SCORE);
    hinge.fasteners.folding_mechanism = Some(MAX_SCORE);
    hinge.tools.folding_mechanism = Some(MAX_SCORE);
    assert!(hinge.scores_are_in_range());
}

#[test]
fn both_article_1_exclusions_round_trip_on_their_own_wire_names() {
    for (exclusion, wire) in [
        (IndexScopeExclusion::RollableDisplay, "rollable-display"),
        (
            IndexScopeExclusion::HighSecurityCommunication,
            "high-security-communication",
        ),
    ] {
        assert_eq!(
            serde_json::to_value(exclusion).expect("serialises"),
            serde_json::Value::String(wire.to_owned())
        );
        let back: IndexScopeExclusion =
            serde_json::from_str(&format!("\"{wire}\"")).expect("round-trips");
        assert_eq!(back, exclusion);
        assert_eq!(exclusion.wire_str(), wire);
    }
    assert_eq!(
        IndexScopeExclusion::ALL.len(),
        2,
        "Art. 1 carves out exactly two product classes"
    );
}

#[test]
fn the_declared_exclusions_agree_with_the_rules_crates_scope_predicate() {
    // The two live in different crates by design and must not drift: the wire
    // strings this type emits are the strings the predicate matches on.
    use dpp_rules::electronics::repairability_index_scope::{
        RepairabilityIndexScope, repairability_index_scope,
    };

    for exclusion in IndexScopeExclusion::ALL {
        let scope = repairability_index_scope("smartphone", Some(exclusion.wire_str()));
        assert!(
            scope.is_declared_exclusion(),
            "{exclusion:?} serialises to {:?}, which the predicate does not recognise",
            exclusion.wire_str()
        );
    }
    assert_eq!(
        repairability_index_scope("smartphone", None),
        RepairabilityIndexScope::Covered
    );
}
