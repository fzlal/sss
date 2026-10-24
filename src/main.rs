use macroquad::prelude::*;

use slime_soccer::net::cli::Cli;

fn window_conf() -> Conf {
    Conf {
        window_title: "Super Slime Soccer".to_owned(),
        window_width: 1120,
        window_height: 800,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match Cli::parse(&args) {
        Ok(cli) => cli,
        Err(error) => {
            eprintln!("{error}\n\n{}", slime_soccer::net::cli::usage());
            std::process::exit(2);
        }
    };
    if cli.help {
        println!("{}", slime_soccer::net::cli::usage());
        return;
    }
    if cli.server {
        slime_soccer::net::server::run_headless(cli);
    }

    let app_cli = cli.clone();
    macroquad::Window::from_config(window_conf(), async move {
        slime_soccer::app::run(app_cli).await;
    });
}
