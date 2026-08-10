//! ChefBar — entrypoint (CLI + GTK-app).
//!
//! Commando's: `chefbar` (app), `chefbar doctor`, `chefbar serve`,
//! `chefbar ipc <cmd>` (externe commando's naar een draaiende instantie),
//! `chefbar --version`.

use chefbar::config::{global_profile, load_profile, set_global_profile};
use chefbar::css;
use chefbar::policy::EndpointPolicy;
use clap::Parser;
use gtk::prelude::*;

#[derive(Parser, Debug)]
#[command(
    name = "chefbar",
    version,
    about = "ChefGroep assistent-app (Rust native)"
)]
struct Cli {
    /// Endpoint-profiel (JSON-pad of via CHEFBAR_ENDPOINT_PROFILE).
    #[arg(long)]
    profile: Option<std::path::PathBuf>,

    /// Doctor-checks uitvoeren en afsluiten.
    #[arg(long)]
    doctor: bool,

    /// Alleen de actor (poll-loop) starten, geen UI.
    #[arg(long)]
    serve: bool,

    /// Extern commando naar een draaiende instantie (panel/bar/refresh/doctor/quit).
    #[arg(long)]
    ipc: Option<String>,

    /// Configuratie afdrukken (profiel + policy-summary, geen secrets).
    #[arg(long)]
    show_config: bool,
}

fn main() {
    let cli = Cli::parse();

    // Externe IPC-commando's hebben geen GTK nodig en geen tweede instantie.
    if let Some(command) = &cli.ipc {
        match chefbar::ipc::parse_command(command) {
            Some(cmd) => match chefbar::ipc::send_command(cmd) {
                Ok(()) => {
                    println!("commando verstuurd: {command}");
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("chefbar: kan commando niet versturen: {e}");
                    std::process::exit(1);
                }
            },
            None => {
                eprintln!("chefbar: onbekend commando '{command}'");
                std::process::exit(2);
            }
        }
    }

    if cli.doctor {
        // Doctor must use the same explicitly selected profile as the runtime.
        let profile = load_profile(cli.profile.as_deref());
        set_global_profile(profile);
        let report = chefbar::doctor::run_checks();
        chefbar::doctor::print_report(&report);
        std::process::exit(if report.ok() { 0 } else { 1 });
    }

    if cli.show_config {
        let profile = load_profile(cli.profile.as_deref());
        println!("profiel  {}", profile.name);
        println!("vault    {}", profile.label("vaultApi"));
        println!("ops      {}", profile.label("opsApi"));
        println!("kater    {}", profile.label("katerWorkspace"));
        println!("vastzet  {}", profile.dashboard);
        std::process::exit(0);
    }

    if cli.serve {
        run_actor_only(&cli);
        return;
    }

    run_app(&cli);
}

fn build_runtime(
    cli: &Cli,
) -> (
    chefbar::http::Client,
    chefbar::http::Client,
    chefbar::state::Shared,
) {
    let profile = load_profile(cli.profile.as_deref());
    set_global_profile(profile.clone());

    let policy = EndpointPolicy::default().with_profile_hosts(&profile.all_urls());
    let vault = chefbar::http::Client::new(&profile.vault_api, policy.clone());
    let ops = chefbar::http::Client::new(&profile.ops_api, policy);

    let shared = chefbar::state::Shared::new();
    let _actor = chefbar::state::spawn_actor(shared.clone(), vault.clone(), ops.clone());
    (vault, ops, shared)
}

fn run_actor_only(cli: &Cli) {
    let (_vault, _ops, _shared) = build_runtime(cli);
    println!("chefbar serve: poll-actor draait (vault 5s, ops 15s). Ctrl-C om te stoppen.");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

fn run_app(cli: &Cli) {
    gtk::init().expect("GTK init mislukt — draait er een display? (DISPLAY/WAYLAND_DISPLAY)");

    let (vault, ops, shared) = build_runtime(cli);
    let profile = global_profile().clone();

    // Signaal CSS op het hele proces (strak-skin, dark default).
    let settings = gtk::Settings::default().expect("GTK-settings");
    let theme = css::detect_theme(&settings);
    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(css::styles_css(&theme).as_bytes())
        .expect("ChefBar-CSS laden mislukt");
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("geen scherm"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let revision = shared.revision.clone();
    let executor = chefbar::actions::Executor {
        vault: vault.clone(),
        ops: ops.clone(),
        profile: profile.clone(),
        revision,
    };

    let panel = chefbar::panel::Panel::new(shared.clone(), executor.clone());
    panel.start_refresh_loop();
    let bar = chefbar::bar::ChefBar::new(shared.clone(), executor.clone());

    // Eén UI-commando-kanaal voor tray + ipc + refresh-loop. De dispatcher
    // draait op de UI-thread (glib-timeout), dus widgets zijn hier veilig.
    let (ui_tx, ui_rx) = std::sync::mpsc::channel::<chefbar::tray::UiCommand>();
    let dispatcher: std::sync::Arc<dyn Fn(chefbar::tray::UiCommand)> =
        std::sync::Arc::new(move |cmd| match cmd {
            chefbar::tray::UiCommand::TogglePanel => panel.toggle(),
            chefbar::tray::UiCommand::ToggleBar => bar.toggle(),
            chefbar::tray::UiCommand::Refresh => chefbar::state::refresh_global(),
            chefbar::tray::UiCommand::Doctor => {
                let report = chefbar::doctor::run_checks();
                chefbar::doctor::run_checks_async(report);
            }
            chefbar::tray::UiCommand::Quit => {
                gtk::main_quit();
            }
        });

    chefbar::tray::start_command_bridge(ui_rx, dispatcher);
    chefbar::ipc::spawn_listener(ui_tx.clone());

    // Tray in eigen thread (ksni).
    let snapshot = shared.snapshot.clone();
    let tray = chefbar::tray::ChefTray::new(snapshot, ui_tx);
    let _tray_service = ksni::TrayService::new(tray).spawn();

    gtk::main();
}
