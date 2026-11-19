// Registers this implementation with the suite, in the one way its
// `vc-test-suite-implementations` package supports: a `localConfig.cjs` in the root
// of the suite checkout. With one present, only the implementations it names run.
//
// `name` is the compose service in `compose.yml` and the name in every output file.
//
// The features are what `dpp-vc` implements today, and the one place that says so.
// The suite skips a row whose feature is `false`, and `check-run.mjs` holds the run
// to this list: every row of a `true` feature must run and get the verdict the suite
// expects, and every row of a `false` feature must be skipped. A change that adds the
// SD-JWT or COSE forms flips its two features here, and the suite must stay green.
module.exports = {
  implementations: [
    {
      name: 'dpp-vc',
      implementation: 'dpp-vc',
      'jose-cose': {
        features: {
          credential_jose: true,
          presentation_jose: true,
          credential_sdjwt: false,
          presentation_sdjwt: false,
          credential_cose: false,
          presentation_cose: false,
        },
      },
    },
  ],
};
