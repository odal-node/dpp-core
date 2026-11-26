# W3C VC-JOSE-COSE test suite harness

The command line that the W3C's
[VC-JOSE-COSE test suite](https://github.com/w3c/vc-jose-cose-test-suite) runs against
`dpp-vc`, the container it runs in, and the check that holds a run of the suite to what
this crate implements.

**No conformance is claimed.** A green run says that `dpp-vc` gives the verdict the
suite expects on each row it ran. The suite is not the specification, and
`docs/architecture/STANDARDS.md` is the one place that says whether a claim is made.

The command line is the `vc-jose-cose-suite` binary of `dpp-tests`, an unpublished member of
the root workspace, in `crates/dpp-tests/src/bin/vc_jose_cose_suite/`. `cargo build --workspace`
builds it and needs neither Node nor Docker; only running the suite does. It takes `dpp-vc` and
`dpp-crypto` by path and calls only their public API.

## What the suite calls

For each row the suite starts the implementation's container once, as in its
`tests/test-util.js`:

```text
<issue|verify> --input /tests/input/<file> --key /tests/input/<key>.json \
  --feature <feature> [--sd '<paths>'] --output /tests/output/<n>-<name>.json
```

and reads `result` from the output file. `success`, `failure` and `indeterminate` are
verdicts. A missing file, or anything else, counts as `error`. The exit status is ignored.

The key file is a `JsonWebKey` verification method with a `publicKeyJwk` and a
`secretKeyJwk`.

## What this harness does

| Command and feature | Calls | Writes |
|---|---|---|
| `issue`, `credential_jose` | `sign_credential(document, key, kid, &[])` | `{"result":"success","data":"<compact JWS>"}` |
| `issue`, `presentation_jose` | `sign_presentation(document, key, kid, &[])` | the same |
| `verify`, `credential_jose` | `verify_credential(token, key, options)` | `{"result":"success"}` |
| `verify`, `presentation_jose` | `verify_presentation(token, key, options)` | the same |
| `issue`, `credential_sdjwt` | `sign_credential_sd_jwt(document, key, kid, &[], paths, None)` | `{"result":"success","data":"<SD-JWT>"}` |
| `issue`, `presentation_sdjwt` | `sign_presentation_sd_jwt(document, key, kid, &[], paths, None)` | the same |
| `verify`, `credential_sdjwt` | `verify_credential_sd_jwt(token, key, options, &binding)` | `{"result":"success"}` |
| `verify`, `presentation_sdjwt` | `verify_presentation_sd_jwt(token, key, options, &binding)` | the same |
| `issue`, `credential_cose` | `sign_credential_cose(document, key, kid, &[])` | `{"result":"success","data":"<base64 of the COSE_Sign1>"}` |
| `issue`, `presentation_cose` | `sign_presentation_cose(document, key, kid, &[])` | the same |
| `verify`, `credential_cose` | `verify_credential_cose(bytes, key, options)` | `{"result":"success"}` |
| `verify`, `presentation_cose` | `verify_presentation_cose(bytes, key, options)` | the same |

A refusal by the library is a `failure`, with the reason in a `reason` member and on
stderr. The process then exits 0: a refusal is the right answer to a row that expects one.

- **The key.** `issue` builds the signing key from `secretKeyJwk`, and `verify` the
  verification key from `publicKeyJwk`.
- **The `kid`.** The key file's `id`, which is the absolute URL of its verification
  method. VC-JOSE-COSE §4.2 asks for an absolute URL when the issuer or holder is identified
  by a URL and there is no `iss`, and the signers hold the `kid` to that, as no verifier does.
  A JOSE or SD-JWT token that carries `iss` may carry a bare `kid` (a credential's `iss` must
  be its issuer). A COSE signer writes no `iss`, and a member named `iss` in the document
  exempts nothing, so a COSE issuance whose issuer or holder is a URL always needs the
  absolute form.
- **No extension terms.** A member outside the VC Data Model 2.0 vocabulary is refused
  at issuance, which is what row 5 expects.
- **The paths.** `--sd` is a JSON array of the suite's dotted paths, with `[n]` for one
  array element: `credentialSubject.phoneNumbers[0]`. The harness turns each into the
  library's `ClaimPath` (`crates/dpp-tests/src/bin/vc_jose_cose_suite/paths.rs`), because the library has no string syntax of its
  own. A path that is not in that syntax is an unusable input, not a verdict, and so is
  `--sd` given to a verification or to a JOSE feature, even as `[]`. An SD-JWT issuance
  without `--sd` conceals nothing.
- **A presentation is verified with `verify_presentation` only.** VC-JOSE-COSE §5.1
  defines verifying a presentation as validating its own JWT, and so for `vp+sd-jwt` with
  `verify_presentation_sd_jwt`, and §5.3 as the `COSE_Sign1` process on the presentation
  itself, so for `vp+cose` with `verify_presentation_cose`. Each judges the presentation's
  envelopes as envelopes, and none verifies what is inside one: of it, each reads only a
  `kid` hint from a `vc+jwt` or `vc+sd-jwt` envelope, unverified. Row 7 envelops a `vc+jwt`, a
  `vc+sd-jwt` and a `vc+cose` credential and expects success, and row 31's one envelope is a
  `vc+jwt` whose data is `<header>.<payload>....` and is no compact JWS, and expects success
  too, so the credentials inside are not verified. `verify_credentials` would verify row 7's
  three under their own keys, and refuse row 31's, whose data cannot be read.
- **COSE files are base64 text.** A COSE input is read as the suite writes its own: the
  standard alphabet with no padding, perhaps with a final newline, and an issued COSE message
  is written the same way, in `data`. Text that is not base64 at all is a refusal and not a
  missing verdict, since it is the input that is wrong, as a JSON document in place of a
  token is. **Row 30 is row 29's bytes written as hex, which is also valid base64**, so
  the text decodes, and what it decodes to is not a `COSE_Sign1`: that is why the library
  refuses it. The harness has no hex detection, and adds none.
- **The token is trimmed.** Some of the suite's token files end in a newline.
- **No audience and no Key Binding.** `options` is `VerifyOptions::new()`, with no
  audience, and `binding` is `KeyBindingRequirement::NotRequired`. None of the suite's
  inputs carries `aud`, `cnf` or a Key Binding JWT, and the suite names no verifier.
- **No time.** `options` carries no `now`, so `exp` and `nbf` are not read. The suite's
  presentations carry an `exp` in December 2024. Under the clock, row 7 would be
  refused for its age, and rows 14 and 16, which expect a refusal for a media type and
  an envelope, would be refused for their age and pass for the wrong reason. The suite
  states no verification time. Held to the clock, the library refuses row 7 from its
  `exp` on (`row_7_is_refused_from_its_exp_on` in
  `crates/dpp-vc/src/jose/suite_tests.rs`), so a report of this run says the run had no
  clock.

An input the harness cannot use, such as a key file that is not a key or a missing file,
is not a verdict. The process exits 2 with the reason on stderr and writes no file, so
the suite scores the row as an `error`. A file left at `--output` by an earlier run is
removed first, so it cannot be read in place of this run's. The same holds for a feature
the suite does not have, which exits 2 by name, and for `--sd` given to anything but the
issuance of an SD-JWT feature, the COSE features included.

## Which features run

`localConfig.cjs` is the one place that says which features are on. It registers the
implementation with the suite in the one way its `vc-test-suite-implementations` package
supports: a `localConfig.cjs` in the root of the suite checkout, with which only the
implementations it names run. All six features are on: `credential_jose`,
`presentation_jose`, `credential_sdjwt`, `presentation_sdjwt`, `credential_cose` and
`presentation_cose`. The suite's `vc+sd-jwt` form is not the IETF SD-JWT VC that
`dpp-vc::sd_jwt_vc` implements.

## How CI runs it

`.github/workflows/vc-jose-cose-suite.yml` runs on every pull request and every push to
`main`, and is not path-filtered. In order, it:

1. formats, lints and tests the harness: `cargo fmt --check -p dpp-tests`, `cargo clippy
   --locked -p dpp-tests --all-targets -- -D warnings`, and `cargo test --locked -p
   dpp-tests --bin vc-jose-cose-suite --test vc_jose_cose_suite_usage_error`;
2. builds the harness on the runner, `cargo build --locked -p dpp-tests --bin
   vc-jose-cose-suite`;
3. checks out the suite at the commit named in the workflow's `SUITE_COMMIT`, the one
   place that names it, and installs it with `npm ci --ignore-scripts`;
4. runs the checker's own tests, `check-run.test.mjs`, against the suite's
   `tests/test-mapping.js`: each case builds a suite root in a temporary directory and
   runs `check-run.mjs` in it, and the step fails if any case gets the wrong exit status
   or message;
5. replaces the suite's `implementations/compose.yml` with `compose.yml`, which lists
   only this implementation and runs the binary in a stock base image pinned by digest,
   and adds `localConfig.cjs`, then pulls that base image;
6. runs the suite with mocha's `json` reporter;
7. runs `check-run.mjs` on that report.

The suite's own exit status is not enough: it passes a run in which every row was
skipped, and it skips a row for any feature its manifest does not declare `true`.
`check-run.mjs` fails unless:

- all six features are declared, each `true` or `false`;
- every row of an enabled feature ran and got the verdict the suite expects;
- every row of a disabled feature was skipped;
- no other implementation, row or failure is in the report;
- at least one row ran;
- an issuance that succeeded left a compact JWS in `data`: three non-empty base64url
  segments, the first of which decodes to a JSON object with a string `alg`. For the
  SD-JWT features it is an SD-JWT: that compact JWS, then its Disclosures, each
  base64url, then a closing `~`. For the COSE features it is the base64 of a `COSE_Sign1`,
  in the standard alphabet with no padding, and `check-run.mjs` reads its CBOR head bytes
  with no dependency: an optional tag 18, an array of four items, which are a byte string
  (the protected header), a map (the unprotected header), a byte string (the payload) and a
  byte string (the signature), and nothing after them. No signature is verified.

The expected number of rows is the number of rows of the enabled features, read from the
suite's own table. Today that is 35 of 35, and nothing here says so.

## Run it locally

Docker and Node 24 are needed: the suite's own mocha does not start under Node 26. The
harness is built on the host and mounted into a Linux container (`compose.yml`), so this
runs on Linux only, with a glibc no newer than the image's (Debian 13, glibc 2.41). On
another system the container cannot execute the binary. From the repository root:

```bash
cargo build --locked -p dpp-tests --bin vc-jose-cose-suite
export SUITE_HARNESS="$PWD/target/debug/vc-jose-cose-suite"

suite=target/vc-jose-cose-test-suite
commit=$(sed -n 's/^  SUITE_COMMIT: //p' .github/workflows/vc-jose-cose-suite.yml)
git init "$suite"
git -C "$suite" fetch --depth 1 https://github.com/w3c/vc-jose-cose-test-suite.git "$commit"
git -C "$suite" checkout FETCH_HEAD
(cd "$suite" && npm ci --ignore-scripts --no-audit --no-fund)

cp .github/oracle/vc-jose-cose/localConfig.cjs "$suite/localConfig.cjs"
cp .github/oracle/vc-jose-cose/compose.yml "$suite/implementations/compose.yml"
rm -f "$suite"/tests/output/*.json

(cd "$suite" && SUITE_UID="$(id -u)" SUITE_GID="$(id -g)" npx mocha tests/ \
  --reporter json --reporter-option output=report.json --timeout 150000 --preserve-symlinks)
(cd "$suite" && node ../../.github/oracle/vc-jose-cose/check-run.mjs report.json)
```

The last command prints each row's verdict beside the one the suite
expects, with a refusal's reason, and the rows skipped. `target/` is ignored by git.

To run the harness by hand:

```bash
cargo test --locked -p dpp-tests --bin vc-jose-cose-suite --test vc_jose_cose_suite_usage_error
cargo run --locked -p dpp-tests --bin vc-jose-cose-suite -- verify --input in.txt \
  --key vm-p256.json --feature credential_jose --output out.json
```

## The container

There is no image to build. `compose.yml` runs the binary the build above wrote,
bind-mounted read-only at `/usr/local/bin/vc-jose-cose-suite` of a stock base image,
Distroless `cc` on Debian 13, which holds glibc and the C runtime and has no shell and no
package manager. `SUITE_HARNESS` is the absolute path of the binary, and the compose file
refuses to start without it. The base image is named by its digest, so the bytes it starts
from are the bytes that were read, and Dependabot does not watch it: moving it is a
deliberate edit, with the tag and the digest changed together. The binary is dynamically
linked, so the base image needs a glibc at least as new as the machine that built it; the
runner's is 2.39 and this image's is 2.41. A Debian 12 base, whose glibc is 2.36, is not
enough.

The suite bind-mounts its `tests/` directory at `/tests`, and the verdict is written
there. Run as the base image's own user (65532), the harness cannot write into a directory
that another user owns, and it says so (`cannot write /tests/output/...: Permission denied`)
and exits 2. Whoever runs it runs it as the user that owns the directory, with `user:` in
the compose file as `compose.yml` does with `SUITE_UID` and `SUITE_GID`, or makes
`tests/output` writable.

## Files

| File | What it is |
|---|---|
| `crates/dpp-tests/src/bin/vc_jose_cose_suite/` | The harness: argument parsing, the feature list, the two commands, and their tests |
| `crates/dpp-tests/tests/vc_jose_cose_suite_usage_error.rs` | A usage error leaves no verdict behind: the binary run as the suite runs it |
| `localConfig.cjs` | The manifest that registers the implementation, and the features on |
| `compose.yml` | Replaces the suite's compose file, with this implementation alone, run in a stock base image |
| `check-run.mjs` | The strict reading of a run |
| `check-run.test.mjs` | The checker's own tests, run by `node --test` |
