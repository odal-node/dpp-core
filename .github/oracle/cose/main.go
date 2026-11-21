// Judges the credentials and presentations `dpp_vc::jose` secures with COSE with an
// independent COSE implementation, and has that implementation sign what `dpp_vc::jose`
// verifies.
//
//	go run . judge <corpus.json> <suite-input-dir>
//	go run . issue <issued.json>
//
// `corpus.json` comes from
// `EMIT_COSE_CORPUS=1 cargo test -p dpp-vc --lib jose::oracle_tests::cose_emit`. See
// `.github/workflows/cose-oracle.yml` for what is checked and what is not.
package main

import (
	"fmt"
	"os"
	"runtime/debug"
)

// libraryVersion is the version of go-cose the module resolved.
func libraryVersion() string {
	if info, ok := debug.ReadBuildInfo(); ok {
		for _, dep := range info.Deps {
			if dep.Path == "github.com/veraison/go-cose" {
				return dep.Version
			}
		}
	}
	return "unknown"
}

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "usage: cose-oracle judge <corpus.json> <suite-input-dir> | issue <issued.json>")
		os.Exit(2)
	}
	suiteDir := os.Getenv("COSE_ORACLE_SUITE_DIR")
	if suiteDir == "" {
		suiteDir = defaultSuiteDir
	}
	keys, err := loadKeys(suiteDir)
	if err != nil {
		fmt.Fprintln(os.Stderr, "cannot load the suite's keys:", err)
		os.Exit(2)
	}
	fmt.Printf("go-cose %s\n", libraryVersion())

	switch os.Args[1] {
	case "judge":
		if len(os.Args) != 4 {
			fmt.Fprintln(os.Stderr, "usage: cose-oracle judge <corpus.json> <suite-input-dir>")
			os.Exit(2)
		}
		r := &Report{}
		controls(r, keys)
		if len(r.Failures) > 0 {
			report(r)
			os.Exit(1)
		}
		fmt.Printf("%d negative controls failed the check each names\n", r.Run)
		cases := judgeCorpus(r, keys, os.Args[2])
		suiteInputs(r, keys, os.Args[3])
		floor(r, cases)
		counts := map[string]int{}
		for _, c := range cases {
			counts[c.Group]++
		}
		fmt.Printf("judged %d messages and %d envelopes (corpus: %d suite, %d credential, %d presentation)\n",
			r.Judged, r.Envelope, counts["suite"], counts["credential"], counts["presentation"])
		if len(r.Failures) > 0 {
			report(r)
			os.Exit(1)
		}
	case "issue":
		if err := issue(keys, os.Args[2]); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
	default:
		fmt.Fprintln(os.Stderr, "unknown command", os.Args[1])
		os.Exit(2)
	}
}

func report(r *Report) {
	fmt.Printf("\n%d failure(s):\n", len(r.Failures))
	for _, f := range r.Failures {
		fmt.Println("  " + f)
	}
}
