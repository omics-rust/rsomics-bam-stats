use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn ours() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rsomics-bam-stats"))
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/small.bam")
}

fn samtools_available() -> bool {
    Command::new("samtools")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Value of an `SN\t<label>:\t<value>` line (both ours and samtools stats use it).
fn sn(out: &str, label: &str) -> i64 {
    let key = format!("SN\t{label}:");
    out.lines()
        .find(|l| l.starts_with(&key))
        .and_then(|l| l.split('\t').nth(2))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or_else(|| panic!("missing SN {label}"))
}

// Core summary counts must match `samtools stats` (ours computes the SN summary;
// label wording differs, so compare mapped fields by value).
#[test]
fn summary_matches_samtools_stats() {
    if !samtools_available() {
        eprintln!("skipping: samtools not found");
        return;
    }
    let ours_out = String::from_utf8(ours().arg(fixture()).output().unwrap().stdout).unwrap();
    let theirs = String::from_utf8(
        Command::new("samtools")
            .arg("stats")
            .arg(fixture())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    // ours "total/mapped reads" count ALL records; samtools "raw total
    // sequences"/"reads mapped" exclude secondary+supplementary. ours reports
    // those separately, so the primary counts are derivable and must match.
    let sec = sn(&ours_out, "secondary");
    let supp = sn(&ours_out, "supplementary");
    assert_eq!(
        sn(&ours_out, "total reads") - sec - supp,
        sn(&theirs, "raw total sequences"),
        "primary total"
    );
    assert_eq!(
        sn(&ours_out, "mapped reads") - sec - supp,
        sn(&theirs, "reads mapped"),
        "primary mapped"
    );

    // "paired reads" and "properly paired" exclude secondary+supplementary in both ours
    // and samtools, so compare directly. "unmapped reads" and "duplicates" are
    // flag-level counts that match verbatim.
    let direct = [
        ("unmapped reads", "reads unmapped"),
        ("paired reads", "reads paired"),
        ("properly paired", "reads properly paired"),
        ("duplicates", "reads duplicated"),
    ];
    for (o, s) in direct {
        assert_eq!(sn(&ours_out, o), sn(&theirs, s), "field `{o}` vs `{s}`");
    }
}
