//! UI-free diff model: parse `git diff` / `gh pr diff` unified patches into a
//! structured model, with word-level intra-line highlights on modified runs.

use imara_diff::intern::InternedInput;
use imara_diff::Algorithm;
use std::ops::Range;

#[derive(Debug, Default)]
pub struct PrDiff {
    pub files: Vec<FileDiff>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Binary,
}

#[derive(Debug)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: FileStatus,
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
}

impl FileDiff {
    pub fn display_path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .unwrap_or("<unknown>")
    }
}

#[derive(Debug)]
pub struct Hunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    /// Function/section context after the trailing `@@`, if any.
    pub section: String,
    pub rows: Vec<DiffRow>,
}

/// A row is only ever Context, Added, or Removed — "modified" exists only at
/// the hunk level, as removed rows followed by added rows.
#[derive(Debug)]
pub enum DiffRow {
    Context {
        old_no: u32,
        new_no: u32,
        text: String,
    },
    Added {
        new_no: u32,
        text: String,
        /// Byte ranges within `text` that differ from the paired removed line.
        intra: Vec<Range<usize>>,
    },
    Removed {
        old_no: u32,
        text: String,
        intra: Vec<Range<usize>>,
    },
}

pub fn parse_patch(patch: &str) -> PrDiff {
    let mut files = Vec::new();
    let mut lines = patch.lines().peekable();

    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("diff --git ") else {
            continue;
        };
        let (old_guess, new_guess) = parse_git_paths(rest);
        let mut file = FileDiff {
            old_path: old_guess,
            new_path: new_guess,
            status: FileStatus::Modified,
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
        };
        let mut is_rename = false;

        // Extended header lines, up to the first hunk or the next file.
        while let Some(next) = lines.peek() {
            if next.starts_with("diff --git ") || next.starts_with("@@ ") {
                break;
            }
            let next = lines.next().unwrap();
            if next.starts_with("new file mode") {
                file.status = FileStatus::Added;
            } else if next.starts_with("deleted file mode") {
                file.status = FileStatus::Deleted;
            } else if let Some(p) = next.strip_prefix("rename from ") {
                file.old_path = Some(p.to_string());
                is_rename = true;
            } else if let Some(p) = next.strip_prefix("rename to ") {
                file.new_path = Some(p.to_string());
                is_rename = true;
            } else if next.starts_with("Binary files ") || next == "GIT binary patch" {
                file.status = FileStatus::Binary;
            } else if let Some(p) = next.strip_prefix("--- ") {
                if let Some(p) = parse_marker_path(p) {
                    file.old_path = Some(p);
                }
            } else if let Some(p) = next.strip_prefix("+++ ") {
                if let Some(p) = parse_marker_path(p) {
                    file.new_path = Some(p);
                }
            }
            // index/mode/similarity lines are ignored.
        }
        if is_rename && file.status == FileStatus::Modified {
            file.status = FileStatus::Renamed;
        }
        if file.status == FileStatus::Added {
            file.old_path = None;
        }
        if file.status == FileStatus::Deleted {
            file.new_path = None;
        }

        while let Some(next) = lines.peek() {
            if !next.starts_with("@@ ") {
                break;
            }
            let header = lines.next().unwrap();
            let Some(mut hunk) = parse_hunk_header(header) else {
                break;
            };
            let mut old_no = hunk.old_start;
            let mut new_no = hunk.new_start;
            while let Some(body) = lines.peek() {
                if body.starts_with("diff --git ") || body.starts_with("@@ ") {
                    break;
                }
                let body = lines.next().unwrap();
                if let Some(text) = body.strip_prefix('+') {
                    hunk.rows.push(DiffRow::Added {
                        new_no,
                        text: text.to_string(),
                        intra: Vec::new(),
                    });
                    new_no += 1;
                    file.additions += 1;
                } else if let Some(text) = body.strip_prefix('-') {
                    hunk.rows.push(DiffRow::Removed {
                        old_no,
                        text: text.to_string(),
                        intra: Vec::new(),
                    });
                    old_no += 1;
                    file.deletions += 1;
                } else if body.starts_with('\\') {
                    // "\ No newline at end of file"
                } else {
                    // Context. Tolerate a bare empty line (some tools strip the
                    // leading space from blank context lines).
                    let text = body.strip_prefix(' ').unwrap_or(body);
                    hunk.rows.push(DiffRow::Context {
                        old_no,
                        new_no,
                        text: text.to_string(),
                    });
                    old_no += 1;
                    new_no += 1;
                }
            }
            compute_intra_line(&mut hunk.rows);
            file.hunks.push(hunk);
        }
        files.push(file);
    }

    PrDiff { files }
}

/// `a/old b/new`, possibly with `"`-quoted paths. Best-effort: the definitive
/// paths come from `---`/`+++`/`rename` lines when present.
fn parse_git_paths(rest: &str) -> (Option<String>, Option<String>) {
    if rest.starts_with('"') {
        let mut parts = Vec::new();
        let mut chars = rest.char_indices();
        while let Some((start, ch)) = chars.next() {
            if ch != '"' {
                continue;
            }
            for (end, ch) in chars.by_ref() {
                if ch == '"' {
                    parts.push(&rest[start + 1..end]);
                    break;
                }
            }
        }
        if parts.len() == 2 {
            return (strip_side(parts[0]), strip_side(parts[1]));
        }
    }
    if let Some(pos) = rest.find(" b/") {
        let old = strip_side(&rest[..pos]);
        let new = strip_side(&rest[pos + 1..]);
        return (old, new);
    }
    (None, None)
}

fn strip_side(path: &str) -> Option<String> {
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .map(str::to_string)
}

/// Path from a `---`/`+++` marker: `a/path`, `b/path`, `/dev/null`, maybe quoted.
fn parse_marker_path(p: &str) -> Option<String> {
    let p = p.trim_end();
    let p = p.strip_prefix('"').unwrap_or(p);
    let p = p.strip_suffix('"').unwrap_or(p);
    if p == "/dev/null" {
        return None;
    }
    strip_side(p).or_else(|| Some(p.to_string()))
}

/// `@@ -old_start[,old_count] +new_start[,new_count] @@[ section]`
fn parse_hunk_header(line: &str) -> Option<Hunk> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let (new, rest) = rest.split_once(" @@")?;
    let parse_pair = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (old_start, old_count) = parse_pair(old)?;
    let (new_start, new_count) = parse_pair(new)?;
    Some(Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
        section: rest.trim_start().to_string(),
        rows: Vec::new(),
    })
}

// --- Intra-line (word-level) diff -------------------------------------------

const MAX_PAIR_RUN: usize = 32;
const MAX_LINE_BYTES: usize = 4096;
const MAX_LINE_TOKENS: usize = 512;
/// If more than this fraction of a line changed, highlighting it all is noise.
const MAX_CHANGED_FRACTION: f32 = 0.7;

/// For each run of removed lines immediately followed by an equal number of
/// added lines, pair the lines positionally and compute word-level diffs.
/// (Equal-count positional pairing is the cheap, high-confidence case;
/// similarity-based pairing for unequal runs can come later.)
fn compute_intra_line(rows: &mut [DiffRow]) {
    let mut i = 0;
    while i < rows.len() {
        if !matches!(rows[i], DiffRow::Removed { .. }) {
            i += 1;
            continue;
        }
        let start = i;
        while i < rows.len() && matches!(rows[i], DiffRow::Removed { .. }) {
            i += 1;
        }
        let mid = i;
        while i < rows.len() && matches!(rows[i], DiffRow::Added { .. }) {
            i += 1;
        }
        let removed = mid - start;
        let added = i - mid;
        if removed != added || removed > MAX_PAIR_RUN {
            continue;
        }
        for pair in 0..removed {
            let old_text = match &rows[start + pair] {
                DiffRow::Removed { text, .. } => text.clone(),
                _ => unreachable!(),
            };
            let new_text = match &rows[mid + pair] {
                DiffRow::Added { text, .. } => text.clone(),
                _ => unreachable!(),
            };
            if old_text.len() > MAX_LINE_BYTES || new_text.len() > MAX_LINE_BYTES {
                continue;
            }
            let (old_ranges, new_ranges) = word_diff(&old_text, &new_text);
            if let DiffRow::Removed { intra, .. } = &mut rows[start + pair] {
                *intra = old_ranges;
            }
            if let DiffRow::Added { intra, .. } = &mut rows[mid + pair] {
                *intra = new_ranges;
            }
        }
    }
}

/// Byte ranges of tokens: identifier/number runs, whitespace runs, or single
/// punctuation chars.
fn token_ranges(s: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut chars = s.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        let class = char_class(ch);
        let mut end = start + ch.len_utf8();
        if class != CharClass::Punct {
            while let Some(&(next_ix, next_ch)) = chars.peek() {
                if char_class(next_ch) != class {
                    break;
                }
                end = next_ix + next_ch.len_utf8();
                chars.next();
            }
        }
        out.push(start..end);
    }
    out
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum CharClass {
    Word,
    Space,
    Punct,
}

fn char_class(ch: char) -> CharClass {
    if ch.is_alphanumeric() || ch == '_' {
        CharClass::Word
    } else if ch.is_whitespace() {
        CharClass::Space
    } else {
        CharClass::Punct
    }
}

/// Word-level diff of two lines → (changed byte ranges in old, in new).
/// Returns empty ranges when the lines are too different for highlights to help.
fn word_diff(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let old_tokens = token_ranges(old);
    let new_tokens = token_ranges(new);
    if old_tokens.len() > MAX_LINE_TOKENS || new_tokens.len() > MAX_LINE_TOKENS {
        return (Vec::new(), Vec::new());
    }

    let mut input = InternedInput::default();
    input.update_before(old_tokens.iter().map(|r| &old[r.clone()]));
    input.update_after(new_tokens.iter().map(|r| &new[r.clone()]));

    let mut old_ranges: Vec<Range<usize>> = Vec::new();
    let mut new_ranges: Vec<Range<usize>> = Vec::new();
    imara_diff::diff(
        Algorithm::Histogram,
        &input,
        |before: Range<u32>, after: Range<u32>| {
            if before.start < before.end {
                let start = old_tokens[before.start as usize].start;
                let end = old_tokens[before.end as usize - 1].end;
                old_ranges.push(start..end);
            }
            if after.start < after.end {
                let start = new_tokens[after.start as usize].start;
                let end = new_tokens[after.end as usize - 1].end;
                new_ranges.push(start..end);
            }
        },
    );

    let changed = |ranges: &[Range<usize>], len: usize| {
        len > 0 && ranges.iter().map(|r| r.len()).sum::<usize>() as f32 / len as f32
            > MAX_CHANGED_FRACTION
    };
    if changed(&old_ranges, old.trim_end().len()) || changed(&new_ranges, new.trim_end().len()) {
        return (Vec::new(), Vec::new());
    }
    (old_ranges, new_ranges)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
diff --git a/src/main.rs b/src/main.rs
index 1111111..2222222 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,5 @@ fn main()
 use std::io;
-let foo = 1;
+let bar = 1;
+let baz = 2;
 println!();
@@ -10,2 +11,2 @@
 // tail
-old line
+new line
diff --git a/README.md b/README.md
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/README.md
@@ -0,0 +1,1 @@
+# hello
\\ No newline at end of file
diff --git a/logo.png b/logo.png
index 4444444..5555555 100644
Binary files a/logo.png and b/logo.png differ
diff --git a/old.rs b/renamed.rs
similarity index 90%
rename from old.rs
rename to renamed.rs
index 6666666..7777777 100644
--- a/old.rs
+++ b/renamed.rs
@@ -1,1 +1,1 @@
-x
+y
";

    #[test]
    fn parses_files_hunks_and_rows() {
        let diff = parse_patch(SAMPLE);
        assert_eq!(diff.files.len(), 4);

        let main = &diff.files[0];
        assert_eq!(main.display_path(), "src/main.rs");
        assert_eq!(main.status, FileStatus::Modified);
        assert_eq!(main.hunks.len(), 2);
        assert_eq!((main.additions, main.deletions), (3, 2));
        assert_eq!(main.hunks[0].section, "fn main()");
        assert_eq!(main.hunks[0].rows.len(), 5);
        match &main.hunks[0].rows[1] {
            DiffRow::Removed { old_no, text, .. } => {
                assert_eq!(*old_no, 2);
                assert_eq!(text, "let foo = 1;");
            }
            other => panic!("expected removed row, got {other:?}"),
        }
        match &main.hunks[0].rows[4] {
            DiffRow::Context { old_no, new_no, .. } => assert_eq!((*old_no, *new_no), (3, 4)),
            other => panic!("expected context row, got {other:?}"),
        }

        let readme = &diff.files[1];
        assert_eq!(readme.status, FileStatus::Added);
        assert_eq!(readme.old_path, None);
        assert_eq!(readme.display_path(), "README.md");

        let logo = &diff.files[2];
        assert_eq!(logo.status, FileStatus::Binary);
        assert_eq!(logo.display_path(), "logo.png");
        assert!(logo.hunks.is_empty());

        let renamed = &diff.files[3];
        assert_eq!(renamed.status, FileStatus::Renamed);
        assert_eq!(renamed.old_path.as_deref(), Some("old.rs"));
        assert_eq!(renamed.new_path.as_deref(), Some("renamed.rs"));
    }

    #[test]
    fn word_diff_highlights_changed_identifier() {
        let (old_ranges, new_ranges) = word_diff("let foo = 1;", "let bar = 1;");
        assert_eq!(old_ranges, vec![4..7]);
        assert_eq!(new_ranges, vec![4..7]);
    }

    #[test]
    fn word_diff_skips_total_rewrites() {
        let (old_ranges, new_ranges) =
            word_diff("completely different content", "nothing shared at all!");
        assert!(old_ranges.is_empty());
        assert!(new_ranges.is_empty());
    }

    #[test]
    fn intra_line_set_on_paired_run() {
        let diff = parse_patch(SAMPLE);
        match &diff.files[0].hunks[1].rows[1] {
            DiffRow::Removed { intra, .. } => assert_eq!(intra, &vec![0..3]),
            other => panic!("expected removed row, got {other:?}"),
        }
        match &diff.files[0].hunks[1].rows[2] {
            DiffRow::Added { intra, .. } => assert_eq!(intra, &vec![0..3]),
            other => panic!("expected added row, got {other:?}"),
        }
    }

    #[test]
    fn hunk_header_without_counts() {
        let hunk = parse_hunk_header("@@ -5 +7 @@").unwrap();
        assert_eq!(
            (hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count),
            (5, 1, 7, 1)
        );
    }
}
