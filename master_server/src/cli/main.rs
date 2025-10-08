use anyhow::Result;

fn main() -> Result<()> {
    println!("Master Server CLI - masterctl");
    println!();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: masterctl <command>");
        println!("Commands:");
        println!("  bootstrap-admin  - Create the first admin user");
        return Ok(());
    }

    match args[1].as_str() {
        "bootstrap-admin" => bootstrap_admin()?,
        _ => {
            println!("Unknown command: {}", args[1]);
            println!("Run 'masterctl' without arguments for usage.");
        }
    }

    Ok(())
}

fn bootstrap_admin() -> Result<()> {
    println!("=== Bootstrap Admin User ===");
    println!();
    println!("This functionality will be implemented in Phase 9.");
    println!("For now, please use SQL to create your first admin user.");

    Ok(())
}
