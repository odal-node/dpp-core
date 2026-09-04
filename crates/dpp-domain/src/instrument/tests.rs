//! `InstrumentCatalog` load, provenance, fold and divergence tests.

use super::*;
use crate::catalog::{CatalogError, Granularity, ProductGroupCatalog, RetentionBasis};

/// Every embedded manifest parses and lands in the catalog.
///
/// Asserted against the embedded table rather than a literal, which went stale
/// the first time an act was added — the same defect this file exists to catch
/// in the manifests themselves.
#[test]
fn loads_all_embedded_manifests() {
    let catalog = InstrumentCatalog::new();
    assert_eq!(catalog.len(), catalog::EMBEDDED_COUNT);
}

/// Every instrument claiming a text must name it. An `Adopted` record with no
/// CELEX is a claim about law with nothing behind it, which is the failure mode
/// this catalog was built to stop; an `Anticipated` one with a CELEX is the
/// mirror error, asserting a text exists when it does not.
#[test]
fn a_claim_to_have_a_text_is_backed_by_a_celex() {
    for instrument in InstrumentCatalog::new().all() {
        assert_eq!(
            instrument.celex.is_some(),
            instrument.status.has_citable_text(),
            "instrument '{}' is {:?} but celex is {:?}",
            instrument.id,
            instrument.status,
            instrument.celex
        );
    }
}

/// An act adopted **under** a framework must say which one; a framework or a
/// direct instrument has nothing above it to name.
///
/// Both delegated and implementing acts are adopted under a basic act, so both
/// carry a parent. The Treaty distinction between them is about what they may
/// do, not about whether they have one.
#[test]
fn acts_adopted_under_a_framework_carry_a_parent() {
    for instrument in InstrumentCatalog::new().all() {
        let expects_parent = matches!(
            instrument.kind,
            InstrumentKind::Delegated | InstrumentKind::Implementing
        );
        assert_eq!(
            instrument.parent.is_some(),
            expects_parent,
            "instrument '{}' kind {:?} parent {:?}",
            instrument.id,
            instrument.kind,
            instrument.parent
        );
    }
}

/// The case the catalog exists for: an act reaching a product group we hold no
/// manifest for. If this ever passes trivially — because someone "tidied up" by
/// deleting the binding or by inventing a manifest to satisfy it — the model has
/// quietly lost the ability to record what a horizontal act covers.
#[test]
fn an_act_may_reach_a_product_group_that_has_no_manifest() {
    let instruments = InstrumentCatalog::new();
    let product_groups = ProductGroupCatalog::new();

    let unmodelled: Vec<&str> = instruments
        .product_group_keys()
        .into_iter()
        .filter(|key| product_groups.get(key).is_none())
        .collect();

    assert!(
        unmodelled.contains(&"lmt"),
        "expected light means of transport to be reached without a manifest, got {unmodelled:?}"
    );
    assert!(
        instruments
            .bindings_for("lmt")
            .iter()
            .all(|(_, b)| !b.allows_determination()),
        "a product group we do not model must not be determinable"
    );
}

/// Binding today and requiring a passport are different questions, and the two
/// instruments that prove it answer them in opposite directions.
#[test]
fn determinability_and_passport_duty_are_independent() {
    let catalog = InstrumentCatalog::new();

    // Adjacent act: obligations bind now, passport is discharged through EPREL.
    assert!(
        !catalog.determinable_for("electronics").is_empty(),
        "electronics ecodesign obligations bind today"
    );
    assert!(
        !catalog.passport_required_for("electronics"),
        "no passport obligation exists for electronics"
    );

    // Framework obligation: binds now, has no passport article at all.
    assert!(!catalog.determinable_for("unsold-goods").is_empty());
    assert!(!catalog.passport_required_for("unsold-goods"));

    // Direct instrument: binds now *and* owes a passport, on a sourced date.
    assert!(!catalog.determinable_for("battery").is_empty());
    assert!(catalog.passport_required_for("battery"));
    let due = catalog.passport_due_for("battery").expect("a fixed date");
    assert_eq!(due.date, "2027-02-18");
    assert_eq!(due.basis, DateBasis::Sourced);

    // Adopted, dated, but nothing determinable until it applies.
    assert!(catalog.determinable_for("toy").is_empty());
    assert!(catalog.passport_required_for("toy"));
}

/// The gate that stops the `electronics` defect being expressible: a
/// determination may be made under the act, and it cannot be a passport
/// determination.
#[test]
fn a_displaced_passport_is_never_reported_as_required() {
    let catalog = InstrumentCatalog::new();
    let instrument = catalog
        .get("ecodesign-energy-labelling-mobile")
        .expect("embedded");

    assert!(matches!(
        instrument.passport_for("electronics"),
        Some(PassportObligation::DisplacedBy { .. })
    ));
    assert!(!instrument.requires_passport_for("electronics"));
    assert!(
        instrument
            .binding("electronics")
            .expect("bound")
            .allows_determination(),
        "the ecodesign obligations themselves are live and must stay determinable"
    );
}

/// An unsourced figure anywhere makes the compound figure unsourced, whichever
/// act supplied the maximum — the safe direction for a claim about someone
/// else's legal obligation.
#[test]
fn retention_folds_to_the_maximum_and_assumption_is_contagious() {
    let catalog = InstrumentCatalog::new();

    // Battery: one contributing act, ten years, explicitly assumed.
    assert_eq!(
        catalog.retention_for("battery"),
        Some((10, RetentionBasis::Assumed))
    );
    // Toy: one contributing act, ten years, sourced from the Regulation.
    assert_eq!(
        catalog.retention_for("toy"),
        Some((10, RetentionBasis::Sourced))
    );
    // Textile inherits ESPR's figure, which the framework itself marks assumed:
    // Art. 9(2)(i) fixes no number, it delegates one to acts that do not exist.
    assert_eq!(
        catalog.retention_for("textile"),
        Some((10, RetentionBasis::Assumed))
    );
    // A product group no act reaches has no figure at all — not a default.
    assert_eq!(catalog.retention_for("packaging"), None);
}

/// No adopted act has fixed a level for anything we carry, and the honest answer
/// is silence. The registry's own item-level default must not leak in here as if
/// it were law.
#[test]
fn granularity_is_unset_because_no_act_has_fixed_one() {
    let catalog = InstrumentCatalog::new();
    for key in catalog.product_group_keys() {
        assert_eq!(
            catalog.granularity_for(key),
            None,
            "product group '{key}' should have no act-fixed granularity yet"
        );
    }
}

#[test]
fn most_granular_wins_when_two_acts_disagree() {
    assert_eq!(
        Granularity::Model.most_granular(Granularity::Item),
        Granularity::Item
    );
    assert_eq!(
        Granularity::Item.most_granular(Granularity::Batch),
        Granularity::Item
    );
}

#[test]
fn an_instrument_id_cannot_be_registered_twice() {
    let mut catalog = InstrumentCatalog::new();
    let duplicate = catalog.get("espr").expect("embedded").clone();
    assert_eq!(
        catalog.register(duplicate),
        Err(CatalogError::AlreadyExists("espr".to_owned()))
    );
}

/// Every passport date we ship traces to an act, and there is nowhere else for
/// one to hide.
///
/// This test replaces a tripwire that pinned the disagreements between the two
/// catalogs while the law still lived on both. It named four product groups. Two
/// of them — `electronics` and `unsold-goods` — carried a `dppAppliesFrom` for a
/// passport obligation that does not exist: `electronics` had Reg. (EU)
/// 2023/1670's *ecodesign* application date, and `unsold-goods` had the ESPR Art.
/// 25 destruction-ban date, from a pair of articles containing no passport at
/// all. Neither field has a home any more, which is the fix.
///
/// What is left to guard is that a date cannot reappear without an act behind it.
#[test]
fn every_passport_date_comes_from_an_act_that_requires_a_passport() {
    let instruments = InstrumentCatalog::new();

    for instrument in instruments.all() {
        for binding in &instrument.product_groups {
            let obligation = binding.passport.as_ref().unwrap_or(&instrument.passport);
            if let Some(date) = obligation.applies_from() {
                assert!(
                    obligation.is_required(),
                    "'{}' under '{}' carries the date {} on an obligation that is not Required",
                    binding.product_group,
                    instrument.id,
                    date.date
                );
                assert!(
                    instrument.status.has_citable_text(),
                    "'{}' fixes a date with no citable text",
                    instrument.id
                );
            }
        }
    }

    // The two that used to over-claim: an act binds them, and no act asks them
    // for a passport, so there is no date to report for either.
    for key in ["electronics", "unsold-goods"] {
        assert!(!instruments.determinable_for(key).is_empty(), "{key} binds");
        assert!(
            !instruments.passport_required_for(key),
            "{key} has no passport obligation under any recorded act"
        );
        assert!(instruments.passport_due_for(key).is_none(), "{key}");
    }
}

/// The live-obligation fold answers correctly in all four quadrants of its two
/// operands, using the real catalog rather than a fixture.
///
/// Both false-positive directions are represented, because they are the reason
/// the fold exists rather than being left to each caller:
///
/// | product group | determinable | passport required | live |
/// |---|---|---|---|
/// | `battery`      | yes | yes | **yes** |
/// | `electronics`  | yes | no — discharged through another system | no |
/// | `unsold-goods` | yes | no — the act has no passport article | no |
/// | `toy`          | no — adopted but not yet applying | yes | no |
///
/// A caller using `determinable_for` alone would enforce a passport for
/// `electronics` and `unsold-goods`, which owe none. One using
/// `passport_required_for` alone would enforce one for `toy`, where nothing is
/// bindingly determinable yet.
#[test]
fn a_live_passport_obligation_needs_both_halves() {
    let catalog = InstrumentCatalog::new();

    assert!(
        catalog.passport_obligation_live("battery"),
        "battery binds today and owes a passport"
    );

    for group in ["electronics", "unsold-goods"] {
        assert!(
            !catalog.determinable_for(group).is_empty(),
            "{group} is determinable, so only the passport half can exclude it"
        );
        assert!(
            !catalog.passport_obligation_live(group),
            "{group} owes no passport, so no obligation is live"
        );
    }

    assert!(
        catalog.passport_required_for("toy"),
        "toy owes a passport, so only the determinability half can exclude it"
    );
    assert!(
        !catalog.passport_obligation_live("toy"),
        "toy's act does not apply yet, so nothing is determinable and no obligation is live"
    );
}

/// A live obligation never claims more than its operands allow, for every
/// product group the catalog knows.
///
/// Stated as two implications rather than by recomputing the conjunction. A test
/// that restates the implementation cannot catch a defect in it, and would fail
/// on exactly the change this fold exists to absorb — the issue behind it notes
/// the conjunction is the piece most likely to grow a third condition. These
/// invariants hold however it grows: whatever else "live" comes to require, it
/// can never be true where no passport is owed, or where nothing is bindingly
/// determinable.
///
/// The other direction — both halves true and the fold saying no — is covered by
/// `battery` above, which is the only combination that produces it.
#[test]
fn a_live_obligation_never_outruns_its_operands() {
    let catalog = InstrumentCatalog::new();
    let product_groups = ProductGroupCatalog::new();

    let mut live = 0_usize;
    for key in product_groups.keys() {
        if !catalog.passport_obligation_live(key) {
            continue;
        }
        live += 1;
        assert!(
            catalog.passport_required_for(key),
            "{key} reports a live obligation while owing no passport"
        );
        assert!(
            !catalog.determinable_for(key).is_empty(),
            "{key} reports a live obligation while nothing is determinable"
        );
    }

    assert!(
        live > 0,
        "no product group reports a live obligation, so the implications above \
         held vacuously and this test proved nothing"
    );
}
