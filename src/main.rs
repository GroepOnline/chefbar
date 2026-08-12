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

    /// Alias voor `--ipc bar` — oude hotkey-bindings (install.sh < 3.1)
    /// roepen `chefbar --bar` aan; zonder deze vlag faalde Super+Space
    /// stil met een clap-error (exit 2).
    #[arg(long)]
    bar: bool,

    /// Configuratie afdrukken (profiel + policy-summary, geen secrets).
    #[arg(long)]
    show_config: bool,
}

fn main() {
    let cli = Cli::parse();

    // Externe IPC-commando's hebben geen GTK nodig en geen tweede instantie.
    // Proberen we een commando te sturen maar is er geen listener, geven we
    // een bruikbare hint (was een stille "kan commando niet versturen").
    let ipc_cmd = if cli.bar { Some("bar".to_string()) } else { cli.ipc.clone() };
    if let Some(command) = &ipc_cmd {
        match chefbar::ipc::parse_command(command) {
            Some(cmd) => match chefbar::ipc::send_command(cmd) {
                Ok(()) => {
                    println!("commando verstuurd: {command}");
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("chefbar: kan commando niet versturen ({e}) — draait er een instantie? (chefbar &)");
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

    // Single-instance: wie de IPC-socket bindt, is dè instantie. Eerst een
    // snelle probe (bestaande instantie shown het paneel — idempotent,
    // Esc verbergt); binden gebeurt vóór GTK-init zodat een gelijktijdige
    // start (on-click/XDG-autostart race) nooit twee panels kan maken.
    if std::env::var("CHEFBAR_FORCE_NEW").is_err() {
        if chefbar::ipc::send_command(chefbar::tray::UiCommand::ShowPanel).is_ok() {
            println!("chefbar: bestaande instantie getoond via IPC");
            std::process::exit(0);
        }
        match chefbar::ipc::acquire() {
            chefbar::ipc::Acquire::Owner(listener) => run_app(&cli, Some(listener)),
            chefbar::ipc::Acquire::Occupied => {
                // De eigenaar is net gestart en nog niet klaar met init;
                // geef het show-commando alsnog door met een korte retry.
                for _ in 0..30 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if chefbar::ipc::send_command(chefbar::tray::UiCommand::ShowPanel).is_ok() {
                        println!("chefbar: bestaande instantie getoond via IPC");
                        std::process::exit(0);
                    }
                }
                eprintln!("chefbar: instantie bezet de socket maar reageert niet; probeer opnieuw");
                std::process::exit(1);
            }
        }
    } else {
        // FORCE_NEW: bewust een tweede instantie — de socket blijft van de
        // eerste (geen listener in dit proces).
        run_app(&cli, None);
    }
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

fn run_app(cli: &Cli, ipc_listener: Option<std::os::unix::net::UnixListener>) {
    gtk::init().expect("GTK init mislukt — draait er een display? (DISPLAY/WAYLAND_DISPLAY)");

    let (vault, ops, shared) = build_runtime(cli);
    let profile = global_profile().clone();

    // Signaal CSS op het hele proces (strak-skin, dark default).
    let settings = gtk::Settings::default().expect("GTK-settings");
    let theme = css::detect_theme(&settings);
    chefbar::tray::set_theme(&theme);
    let provider = gtk::CssProvider::new();
    if let Err(err) = provider.load_from_data(css::styles_css(&theme).as_bytes()) {
        eprintln!("[chefbar] CSS-load mislukt (fallback naar systeemthema): {err}");
    }
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

    // Eén UI-commando-kanaal voor tray + ipc + refresh-loop. De dispatcher
    // draait op de UI-thread (glib-timeout), dus widgets zijn hier veilig.
    let (ui_tx, ui_rx) = std::sync::mpsc::channel::<chefbar::tray::UiCommand>();
    // De closure vangt GTK-widgets (Rc, niet Send/Sync) maar verlaat de
    // UI-thread nooit: de glib-bridge dispatcht alleen op de main-loop.
    #[allow(clippy::arc_with_non_send_sync)]
    let dispatcher: std::sync::Arc<dyn Fn(chefbar::tray::UiCommand)> =
        std::sync::Arc::new(move |cmd| match cmd {
            chefbar::tray::UiCommand::ShowPanel => panel.show(),
            chefbar::tray::UiCommand::TogglePanel => panel.toggle(),
            chefbar::tray::UiCommand::Refresh => chefbar::state::refresh_global(),
            chefbar::tray::UiCommand::Doctor => {
                let report = chefbar::doctor::run_checks();
                chefbar::doctor::run_checks_async(report);
            }
            chefbar::tray::UiCommand::Quit => {
                panel.flush_panel_state();
                gtk::main_quit();
            }
        });

    chefbar::tray::start_command_bridge(ui_rx, dispatcher);
    if let Some(listener) = ipc_listener {
        chefbar::ipc::spawn_listener_on(listener, ui_tx.clone());
    }

    // Tray in eigen thread (ksni).
    let snapshot = shared.snapshot.clone();
    let tray = chefbar::tray::ChefTray::new(snapshot, ui_tx);
    let tray_service = ksni::TrayService::new(tray);
    chefbar::tray::register_handle(tray_service.handle());
    tray_service.spawn();

    gtk::main();
}
