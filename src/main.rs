fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().skip(1).collect::<Vec<_>>() != ["native-info"] {
        eprintln!(
            "Usage: hotr native-info\nOwner lifecycle and client access are not implemented yet."
        );
        std::process::exit(2);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&hotr::linked_native_versions()?)?
    );
    Ok(())
}
