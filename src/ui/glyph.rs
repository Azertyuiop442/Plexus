
use nf_icons::nf;

pub fn resolve_glyph(name: &str) -> &str {
    let clean = name.trim().to_lowercase();
    match clean.as_str() {
        "nf-dev-git" | "dev-git" | "git" => nf!("nf-dev-git"),
        "nf-oct-git_branch"
        | "oct-git_branch"
        | "nf-dev-git_branch"
        | "dev-git_branch"
        | "branch"
        | "branches" => nf!("nf-oct-git_branch"),
        "nf-oct-checklist"
        | "oct-checklist"
        | "nf-cod-checklist"
        | "cod-checklist"
        | "files"
        | "file"
        | "status"
        | "checklist" => nf!("nf-oct-checklist"),
        "nf-oct-history"
        | "oct-history"
        | "nf-cod-history"
        | "cod-history"
        | "log"
        | "history" => nf!("nf-oct-history"),
        "nf-oct-archive" | "oct-archive" | "stash" | "archive" => nf!("nf-oct-archive"),
        "nf-oct-globe" | "oct-globe" | "remotes" | "remote" | "globe" => nf!("nf-oct-globe"),
        "nf-cod-add"
        | "cod-add"
        | "nf-oct-plus"
        | "oct-plus"
        | "stage"
        | "add"
        | "+"
        | "plus" => nf!("nf-cod-add"),
        "nf-oct-file_diff" | "oct-file_diff" | "nf-cod-diff" | "cod-diff" | "diff" => {
            nf!("nf-oct-file_diff")
        }
        "nf-oct-diff" | "oct-diff" | "unstage" => nf!("nf-oct-diff"),
        "nf-oct-git_commit"
        | "oct-git_commit"
        | "nf-dev-git_commit"
        | "dev-git_commit"
        | "commit" => nf!("nf-oct-git_commit"),
        "nf-cod-folder_opened" | "cod-folder_opened" | "folder" => nf!("nf-cod-folder_opened"),
        "nf-cod-close" | "cod-close" | "close" => nf!("nf-cod-close"),

        _ if name.chars().count() == 1 => name,
        _ => "",
    }
}

