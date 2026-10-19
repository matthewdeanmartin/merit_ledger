//! Merit Ledger firmware for the Waveshare ESP32-S3-Touch-LCD-1.47. Build
//! with `make firmware`; read ../DEPLOY.md before flashing.
//!
//! Tasks: `BoardConfig::s3` puts TLS handshakes on core 0 and the
//! connection loop on core 1; this main task runs housekeeping. A failed
//! startup is explained at http://<board>:8080/ once Wi-Fi is up.
#[cfg(not(target_os = "espidf"))]
compile_error!("build the firmware with `make firmware` (xtensa-esp32s3-espidf, --features esp32)");

use esp_idf_svc::hal::peripherals::Peripherals;
use meritledger::{App, Limits};
use miniframework::esp::{self, BoardConfig};
use miniframework::status::SIGNALS;
use miniframework::{Config, Site};
use std::sync::Arc;

mod bundle {
    use miniframework::web::Asset;
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
    pub fn assets() -> &'static [Asset] {
        ASSETS
    }
}

const HOSTNAME: &str = match option_env!("MERITLEDGER_HOSTNAME") {
    Some(h) => h,
    None => "meritledger",
};

fn main() {
    if let Err(e) = run() {
        esp::fail(&e.to_string());
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    esp::init();
    let ssid = option_env!("MERITLEDGER_WIFI_SSID")
        .ok_or("MERITLEDGER_WIFI_SSID missing at build time")?;
    let password = option_env!("MERITLEDGER_WIFI_PASSWORD")
        .ok_or("MERITLEDGER_WIFI_PASSWORD missing at build time")?;
    let peripherals = Peripherals::take()?;

    let mut board_config = BoardConfig::s3(ssid, password, HOSTNAME);
    board_config.instance = "Merit Ledger";
    // The household CA, for outbound HTTPS to household hosts (mastomini).
    board_config.ca_pem =
        Some(concat!(include_str!("../../certs/household-ca.crt"), "\0").as_bytes());
    #[cfg(feature = "https")]
    {
        board_config.cert_pem =
            concat!(include_str!(concat!(env!("OUT_DIR"), "/server.crt")), "\0").as_bytes();
        board_config.key_pem =
            concat!(include_str!(concat!(env!("OUT_DIR"), "/server.key")), "\0").as_bytes();
    }
    board_config.wait_for_wifi = false;
    let board = esp::start(board_config, peripherals.modem)?;

    SIGNALS.step(8, "sd card");
    let sd = Arc::new(card::mount(
        peripherals.sdmmc1,
        peripherals.pins.gpio15,
        peripherals.pins.gpio16,
        peripherals.pins.gpio17,
        peripherals.pins.gpio18,
        peripherals.pins.gpio13,
        peripherals.pins.gpio14,
    ));

    SIGNALS.step(9, "state partition");
    // The snapshot slots live in their own NVS partition, apart from Wi-Fi
    // and framework settings. Never erased by firmware.
    let kv = Arc::new(board.partition_kv("mlstate", "ml")?);
    let limits = Limits::board();
    // What a PIN check costs on this chip (the rounds target ~100 ms).
    let started = std::time::Instant::now();
    meritledger::pin::hash("0000", limits.pin_rounds)?;
    log::info!(
        "PIN hash: {} rounds in {} ms",
        limits.pin_rounds,
        started.elapsed().as_millis()
    );
    let mut app = App::open(kv, sd, limits)?;
    // What a year of all three calendars costs on this chip (Sprint 7): the
    // first time (the f64 astronomy is emulated here) and again from the cache.
    let tz = meritledger::tz::Tz::parse(meritledger::state::DEFAULT_TZ)?;
    let from = meritledger::tz::Date {
        year: 2026,
        month: 1,
        day: 1,
    };
    let to = meritledger::tz::Date {
        year: 2026,
        month: 12,
        day: 31,
    };
    let year_of_three = || {
        [
            meritledger::calendar::System::Theravada,
            meritledger::calendar::System::Chinese,
            meritledger::calendar::System::Japanese,
        ]
        .iter()
        .map(|s| meritledger::calendar::observances(*s, Default::default(), &tz, from, to).len())
        .sum::<usize>()
    };
    let started = std::time::Instant::now();
    let n = year_of_three();
    let cold = started.elapsed().as_millis();
    let started = std::time::Instant::now();
    year_of_three();
    log::info!(
        "calendar: a year of three calendars ({n} observances): {cold} ms first, {} us cached",
        started.elapsed().as_micros()
    );
    app.build = option_env!("MERITLEDGER_BUILD").unwrap_or("dev");
    app.setup_code = option_env!("MERITLEDGER_SETUP_CODE");

    let mut config = Config::new("meritledger", format!("{HOSTNAME}.local"));
    config.assets = bundle::assets();
    config.ca_der = Some(include_bytes!("../../certs/household-ca.der"));
    config.proto_package = "meritledger";
    // Calendar feeds are answered by the app, not static files.
    config.app_paths = &["/cal"];
    // What /.well-known/miniframework says (homelab and friends read it).
    config.about = miniframework::about::AboutConfig {
        title: "Merit Ledger",
        description: "Household Buddhist practice ledger",
        board: "Waveshare ESP32-S3-Touch-LCD-1.47",
        hardware: &["screen:320x172", "touch", "sd-slot", "button:boot"],
        links: &[],
    };
    config.build = app.build;
    app.dev_origins = config.origins.clone();

    #[cfg(feature = "gate")]
    start_gate(&board, &app)?;

    let site = Site::new(config, app, board.platform(peripherals.temp_sensor));
    log::info!("Merit Ledger ready: http://{HOSTNAME}.local/ (joining Wi-Fi)");
    board.serve(site, |site| site.service.tick())
}

/// The SD card, 4-bit SDMMC (pins from the board skill file), FAT mounted at
/// `/sd`. No card, or one that won't mount, leaves the board running
/// without one: history is then ephemeral (spec/v2_decisions.md).
mod card {
    use esp_idf_svc::fs::fatfs::Fatfs;
    use esp_idf_svc::hal::gpio::{AnyIOPin, InputPin, OutputPin};
    use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver, SDMMC1};
    use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
    use esp_idf_svc::io::vfs::MountedFatfs;
    use esp_idf_svc::sys;
    use meritledger::sd::Sd;
    use std::path::Path;

    const MOUNT: &str = "/sd";

    #[allow(clippy::too_many_arguments)]
    pub fn mount(
        slot: SDMMC1<'static>,
        cmd: impl OutputPin + 'static,
        clk: impl OutputPin + 'static,
        d0: impl InputPin + OutputPin + 'static,
        d1: impl InputPin + OutputPin + 'static,
        d2: impl InputPin + OutputPin + 'static,
        d3: impl InputPin + OutputPin + 'static,
    ) -> Sd {
        let mounted = (|| -> Result<(), sys::EspError> {
            let host = SdMmcHostDriver::new_4bits(
                slot,
                cmd,
                clk,
                d0,
                d1,
                d2,
                d3,
                None::<AnyIOPin>,
                None::<AnyIOPin>,
                &SdMmcHostConfiguration::new(),
            )?;
            let card = SdCardDriver::new_mmc(host, &SdCardConfiguration::new())?;
            let fatfs = Fatfs::new_sdcard(0, card)?;
            // Mounted for the life of the firmware.
            std::mem::forget(MountedFatfs::mount(fatfs, MOUNT, 4)?);
            Ok(())
        })();
        match mounted {
            Ok(()) => {
                // A card that is present but unformatted mounts, then fails
                // here: treat it as absent rather than serve errors later.
                match std::fs::create_dir_all(format!("{MOUNT}/ml")) {
                    Ok(()) => {
                        log::info!("SD card mounted at {MOUNT}: {:?}", space(Path::new(MOUNT)));
                        Sd::at(MOUNT, Some(space))
                    }
                    Err(e) => {
                        log::warn!("SD card mounted but not writable ({e}); running without it");
                        Sd::none()
                    }
                }
            }
            Err(e) => {
                log::warn!("no SD card ({e}); history beyond the snapshot is ephemeral");
                Sd::none()
            }
        }
    }

    /// Total and free bytes from FATFS.
    fn space(path: &Path) -> Option<(u64, u64)> {
        let path = std::ffi::CString::new(path.to_str()?).ok()?;
        let (mut total, mut free) = (0u64, 0u64);
        // SAFETY: reads the mounted volume's counters into our locals.
        let err = unsafe { sys::esp_vfs_fat_info(path.as_ptr(), &mut total, &mut free) };
        (err == sys::ESP_OK).then_some((total, free))
    }
}

/// Sprint 1 gate: a worker that fetches a household-CA host and a public-CA
/// host in turn, and appends to the SD card, while `POST /api/v1/gate` has
/// it running.
#[cfg(feature = "gate")]
fn start_gate(board: &esp::Board, app: &App) -> Result<(), Box<dyn std::error::Error>> {
    use miniframework::fetch::Fetch;
    let household = option_env!("MERITLEDGER_GATE_HOUSEHOLD_URL")
        .unwrap_or("https://gamematcher.local/api/v1/status");
    let public = option_env!("MERITLEDGER_GATE_PUBLIC_URL")
        .unwrap_or("https://mastodon.social/.well-known/oauth-authorization-server");
    app.gate.set_targets(&[
        ("household-ca", household),
        ("public-ca", public),
        ("sd-append", "sd:ml/gate/append.log"),
    ]);
    let gate = app.gate.clone();
    let sd = app.sd.clone();
    let household_fetch = board.fetch();
    let public_fetch = board.fetch_public();
    esp::spawn_worker(c"ml-gate", 16 * 1024, false, move || {
        let household = move |url: &str| {
            household_fetch
                .get(url, "application/json", 32 * 1024)
                .map(|f| (f.status, f.body.len()))
        };
        let public = move |url: &str| {
            public_fetch
                .get(url, "application/json", 32 * 1024)
                .map(|f| (f.status, f.body.len()))
        };
        let block = [b'm'; 4096];
        let card = move |_: &str| {
            sd.append("ml/gate/append.log", &block)
                .map(|()| (200u16, block.len()))
        };
        loop {
            gate.run_once(&[&household, &public, &card]);
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    })?;
    Ok(())
}
