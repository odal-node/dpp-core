# W3C VC-JOSE-COSE test suite harness

The command line that the W3C's
[VC-JOSE-COSE test suite](https://github.com/w3c/vc-jose-cose-test-suite) runs against
`dpp-vc`, the image it is built into, and the check that holds a run of the suite to what
this crate implements.

**No conformance is claimed.** A green run says that `dpp-vc` gives the verdict the
suite expects on each row it ran. The suite is not the specification, and
`docs/architecture/STANDARDS.md` is the one place that says whether a claim is made.

This is a standalone crate with its own `[workspace]` and lockfile, like `fuzz/`, so
`cargo build --workspace` at the repository root neither builds it nor needs Node or
Docker. It takes `dpp-vc` and `dpp-crypto` by path and calls only their public API.

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
| `verify`, `credential_jose` | `verify_credential(token, key, None)` | `{"result":"success"}` |
| `verify`, `presentation_jose` | `verify_presentation(token, key, None)` | the same |
| `issue`, `credential_sdjwt` | `sign_credential_sd_jwt(document, key, kid, &[], paths)` | `{"result":"success","data":"<SD-JWT>"}` |
| `issue`, `presentation_sdjwt` | `sign_presentation_sd_jwt(document, key, kid, &[], paths)` | the same |
| `verify`, `credential_sdjwt` | `verify_credential_sd_jwt(token, key, None)` | `{"result":"success"}` |
| `verify`, `presentation_sdjwt` | `verify_presentation_sd_jwt(token, key, None)` | the same |

A refusal by the library is a `failure`, with the reason in a `reason` member and on
stderr. The process then exits 0: a refusal is the right answer to a row that expects one.

- **The key.** `issue` builds the signing key from `secretKeyJwk`, and `verify` the
  verification key from `publicKeyJwk`.
- **The `kid`.** The key file's `id`, which is the absolute URL of its verification
  method. VC-JOSE-COSE §4.2 asks for an absolute URL when the document has no `iss`.
- **No extension terms.** A member outside the VC Data Model 2.0 vocabulary is refused
  at issuance, which is what row 5 expects.
- **The paths.** `--sd` is a JSON array of the suite's dotted paths, with `[n]` for one
  array element: `credentialSubject.phoneNumbers[0]`. The harness turns each into the
  library's `ClaimPath` (`src/paths.rs`), because the library has no string syntax of its
  own. A path that is not in that syntax is an unusable input, not a verdict, and so is
  `--sd` given to a verification or to a JOSE feature, even as `[]`. An SD-JWT issuance
  without `--sd` conceals nothing.
- **A presentation is verified with `verify_presentation` only.** VC-JOSE-COSE §5.1
  defines verifying a presentation as validating its own JWT, and so for `vp+sd-jwt` with
  `verify_presentation_sd_jwt`. Row 7 envelops a
  `vc+sd-jwt` and a `vc+cose` credential and expects success, so the credentials inside
  are not verified: calling `verify_credentials` would refuse that presentation.
- **The token is trimmed.** Some of the suite's token files end in a newline.
- **No time.** `now` is `None`, so `exp` and `nbf` are not read. The suite's
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
that is not implemented: `credential_cose` and `presentation_cose` exit 2 by name. They
are never answered with `success` or `failure`, so a manifest that turns one on before it
is built fails loudly.

## Which features run

`localConfig.cjs` is the one place that says which features are on. It registers the
implementation with the suite in the one way its `vc-test-suite-implementations` package
supports: a `localConfig.cjs` in the root of the suite checkout, with which only the
implementations it names run. Today `credential_jose`, `presentation_jose`,
`credential_sdjwt` and `presentation_sdjwt` are on, and the COSE features are off. The
suite's `vc+sd-jwt` and `vc+cose` forms are not the IETF SD-JWT VC that
`dpp-vc::sd_jwt_vc` implements.

A change that builds the COSE form sets its two features to `true` here, adds them to
`src/feature.rs`, and must leave the suite green.

## How CI runs it

`.github/workflows/vc-jose-cose-suite.yml` runs on every pull request and every push to
`main`, and is not path-filtered. In order, it:

1. formats, lints and tests this crate, which no other job reaches;
2. builds the image from the checkout, `vc-jose-cose-suite:local`;
3. checks out the suite at the commit named in the workflow's `SUITE_COMMIT`, the one
   place that names it, and installs it with `npm ci --ignore-scripts`;
4. runs the checker's own tests, `check-run.test.mjs`, against the suite's
   `tests/test-mapping.js`: each case builds a suite root in a temporary directory and
   runs `check-run.mjs` in it, and the step fails if any case gets the wrong exit status
   or message;
5. replaces the suite's `implementations/compose.yml` with `compose.yml`, which lists
   only this implementation and never pulls an image, and adds `localConfig.cjs`;
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
  base64url, then a closing `~`. No signature is verified.

The expected number of rows is the number of rows of the enabled features, read from the
suite's own table. Today that is 26 of 35, and nothing here says so.

`.github/workflows/vc-jose-cose-image.yml` builds the same image from a pushed `v*`
tag and pushes it to `ghcr.io/<owner>/vc-jose-cose-suite`, tagged with the version.
A second job then points `latest` at the highest `vMAJOR.MINOR.PATCH` release whose
image is published, retagging it in the registry without a rebuild. It does not point
`latest` at its own release, so a rerun of an older release cannot move `latest` back,
and it does not matter which of several queued runs survives.

## Run it locally

Docker and Node 22 or later are needed. From the repository root:

```bash
docker build --file w3c-suites/vc-jose-cose/Dockerfile --tag vc-jose-cose-suite:local .

suite=target/vc-jose-cose-test-suite
commit=$(sed -n 's/^  SUITE_COMMIT: //p' .github/workflows/vc-jose-cose-suite.yml)
git init "$suite"
git -C "$suite" fetch --depth 1 https://github.com/w3c/vc-jose-cose-test-suite.git "$commit"
git -C "$suite" checkout FETCH_HEAD
(cd "$suite" && npm ci --ignore-scripts --no-audit --no-fund)

cp w3c-suites/vc-jose-cose/localConfig.cjs "$suite/localConfig.cjs"
cp w3c-suites/vc-jose-cose/compose.yml "$suite/implementations/compose.yml"
rm -f "$suite"/tests/output/*.json

(cd "$suite" && SUITE_UID="$(id -u)" SUITE_GID="$(id -g)" npx mocha tests/ \
  --reporter json --reporter-option output=report.json --timeout 150000 --preserve-symlinks)
(cd "$suite" && node ../../w3c-suites/vc-jose-cose/check-run.mjs report.json)
```

The last command prints each row's verdict beside the one the suite
expects, with a refusal's reason, and the rows skipped. `target/` is ignored by git.

To run the harness by hand:

```bash
cd w3c-suites/vc-jose-cose
cargo test --locked
cargo run --locked -- verify --input in.txt --key vm-p256.json \
  --feature credential_jose --output out.json
```

## The image

A multi-stage build whose context is the repository root, with both base images pinned
by digest: the toolchain `rust-toolchain.toml` pins to build, and Distroless `cc` to run,
which has no shell and no package manager. The entrypoint is the harness binary, and it
runs as user 65532.

The suite bind-mounts its `tests/` directory at `/tests`, and the verdict is written
there. Run as 65532, the harness cannot write into a directory that another user owns, and
it says so (`cannot write /tests/output/...: Permission denied`) and exits 2. Whoever
runs the image runs it as the user that owns the directory, with `docker run --user` or
`user:` in a compose file as `compose.yml` does, or makes `tests/output` writable.

## Files

| File | What it is |
|---|---|
| `src/` | The harness: argument parsing, the feature list, the two commands, and their tests |
| `localConfig.cjs` | The manifest that registers the implementation, and the features on |
| `compose.yml` | Replaces the suite's compose file, with this implementation alone |
| `check-run.mjs` | The strict reading of a run |
| `check-run.test.mjs` | The checker's own tests, run by `node --test` |
| `Dockerfile`, `Dockerfile.dockerignore` | The image, and the paths its build may read |
