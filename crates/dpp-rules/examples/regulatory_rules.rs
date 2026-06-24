//! Example: pure EU ESPR cross-field regulatory rules (no I/O, no allocation
//! beyond the inputs). These are the same rule kernels shared by `dpp-domain`
//! validators and the Wasm sector plugins.
//!
//! Run with: `cargo run --example regulatory_rules -p dpp-rules`

use dpp_rules::batteries::chemistry::mercury_content_prohibited;
use dpp_rules::batteries::recycled_content::{RecycledContentInput, annex_x_shortfalls_2031};
use dpp_rules::{
    FibreInput, SvhcInput, check_svhc_declarations, country_code_valid, fibre_sum_ok,
    validate_fibre_composition, validate_svhc_substances,
};

fn main() {
    println!("=== Textile: fibre composition (must sum to ~100%) ===\n");

    let good = [
        FibreInput {
            fibre: "cotton",
            pct: 70.0,
            country_of_origin: Some("IN"),
        },
        FibreInput {
            fibre: "recycled_polyester",
            pct: 30.0,
            country_of_origin: Some("DE"),
        },
    ];
    println!("  sum_ok(70 + 30) = {}", fibre_sum_ok(&[70.0, 30.0]));
    println!(
        "  validate(good)  = {:?}",
        validate_fibre_composition(&good)
    );

    let bad = [FibreInput {
        fibre: "cotton",
        pct: 80.0,
        country_of_origin: None,
    }];
    println!(
        "  validate(80% only) = {:?}",
        validate_fibre_composition(&bad)
    );

    println!("\n=== Chemicals: SVHC declarations (REACH Art. 33, 0.1% w/w) ===\n");

    let svhcs = [
        SvhcInput {
            cas_number: "80-05-7",
            substance_name: "Bisphenol A",
            concentration_pct: 0.15,
        },
        SvhcInput {
            cas_number: "117-81-7",
            substance_name: "DEHP",
            concentration_pct: 0.05,
        },
    ];
    println!(
        "  structural validation = {:?}",
        validate_svhc_substances(&svhcs)
    );
    for finding in check_svhc_declarations(&svhcs) {
        println!(
            "  finding[{}]: {} ({}) at {:.2}% -> {:?}",
            finding.index,
            finding.substance_name,
            finding.cas_number,
            finding.concentration_pct,
            finding.kind
        );
    }

    println!("\n=== Battery: Annex X recycled-content targets (from 2031) ===\n");

    let recycled = RecycledContentInput {
        cobalt_pct: Some(10.0), // below the 16% Phase-1 target
        lithium_pct: Some(6.0),
        nickel_pct: Some(6.0),
        lead_pct: Some(85.0),
    };
    let shortfalls = annex_x_shortfalls_2031(&recycled);
    if shortfalls.is_empty() {
        println!("  all declared metals meet Phase-1 targets");
    } else {
        for s in shortfalls {
            println!(
                "  shortfall: {} declared {:.1}% < required {:.1}%",
                s.material, s.declared_pct, s.required_pct
            );
        }
    }
    println!(
        "  mercury 0.0010% prohibited? {}",
        mercury_content_prohibited(0.0010)
    );

    println!("\n=== Common: ISO 3166-1 alpha-2 country codes ===\n");
    for code in ["DE", "in", "XX", "USA"] {
        println!(
            "  country_code_valid({code:>3}) = {}",
            country_code_valid(code)
        );
    }
}
