#![cfg_attr(all(not(debug_assertions), feature = "desktop"), windows_subsystem = "windows")]

fn main() {
    t_books_lib::apply_debug_flag();

    #[cfg(feature = "desktop")]
    {
        t_books_lib::run_desktop();
        return;
    }

    #[cfg(not(feature = "desktop"))]
    {
        match t_books_lib::open_db() {
            Ok(_) => {
                println!("T Books local books ready.");
                println!("{}", t_books_lib::db_path().display());
            }
            Err(err) => {
                eprintln!("Could not open the local books: {err}");
                std::process::exit(1);
            }
        }
    }
}
