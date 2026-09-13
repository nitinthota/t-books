//! Owner hive tools: print the locked plan, provision tabs, migrate from the archive.

fn main() {
    t_books_lib::apply_debug_flag();
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "plan".into());
    match cmd.as_str() {
        "plan" => match t_books_lib::validate_plan() {
            Ok(folders) => {
                println!("Sheet hive plan is valid.");
                println!("Voucher_Raw_Data is read-only.");
                for line in folders {
                    println!("{line}");
                }
            }
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        },
        "provision" => match t_books_lib::provision_hive(true) {
            Ok(report) => {
                if let Some(err) = report.error {
                    eprintln!("{err}");
                    for line in report.folders {
                        println!("{line}");
                    }
                    std::process::exit(1);
                }
                for line in report.folders {
                    println!("{line}");
                }
                for line in report.tabs {
                    println!("{line}");
                }
            }
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        },
        "migrate" => {
            let dry = args.next().as_deref() == Some("--dry-run")
                || std::env::args().any(|a| a == "--dry-run");
            match t_books_lib::migrate_from_raw(dry) {
                Ok(plan) => {
                    println!(
                        "imported={} skipped={} payments={} vendors={} jobs={} log={}",
                        plan.imported,
                        plan.skipped,
                        plan.payments_out,
                        plan.vendors.len(),
                        plan.jobs.len(),
                        plan.log.len()
                    );
                    if dry {
                        println!("Dry run. Voucher_Raw_Data was not written.");
                    }
                }
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("Unknown command {other}. Use plan | provision | migrate [--dry-run]");
            std::process::exit(1);
        }
    }
}
