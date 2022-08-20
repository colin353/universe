pub fn print_diff(files: &[service::FileDiff]) {
    // Print added files
    let mut first = true;
    for file in files.iter().filter(|f| f.kind == service::DiffKind::Added) {
        if first {
            first = false;
            println!("Added: ");
        }
        println!("    {} [+{}]", file.path, 325);
    }

    // Print modified files
    let mut first = true;
    for file in files
        .iter()
        .filter(|f| f.kind == service::DiffKind::Modified)
    {
        if first {
            first = false;
            println!("Modified: ");
        }
        println!("    {} [+{} -{}]", file.path, 2, 6);
    }

    // Print deleted files
    let mut first = true;
    for file in files
        .iter()
        .filter(|f| f.kind == service::DiffKind::Removed)
    {
        if first {
            first = false;
            println!("Deleted: ");
        }
        println!("    {} [-{}]", file.path, 64);
    }
}
