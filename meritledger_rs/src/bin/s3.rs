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
    // The screen first, so it shows something while Wi-Fi joins.
    let mut screen = display::Screen::new(
        peripherals.i2c0,
        peripherals.pins.gpio42,
        peripherals.pins.gpio41,
        peripherals.pins.gpio48,
        peripherals.pins.gpio47,
    );
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
    let mut app = App::open(kv.clone(), sd, limits)?;
    app.origin = format!("https://{HOSTNAME}.local");
    // DMs (the household bot on mastomini) and email (Brevo), Sprint 10: a
    // bounded queue in the Kv, sent by a worker so a slow TLS handshake to
    // an instance never stalls serving. Jobs wait for the clock: their
    // retry times are wall-clock.
    let outbox = Arc::new(meritledger::outbound::Outbox::open(
        kv.clone(),
        Arc::new(board.fetch()),
        Arc::new(board.fetch_public()),
    ));
    // Mastodon sign-in and linking (Sprint 11): registered with
    // `write:statuses` too, for publishing as the person; its network steps
    // run on the outbox's worker.
    let oauth = Arc::new(miniframework::oauth::OAuth::new(
        miniframework::oauth::Config {
            app_name: "Merit Ledger",
            website: format!("https://{HOSTNAME}.local"),
            extra_scopes: "write:statuses",
            ..Default::default()
        },
        kv,
        Arc::new(board.fetch()),
        Arc::new(board.fetch_public()),
    ));
    oauth.set_providers(app.instances.clone());
    outbox.attach_oauth(oauth.clone());
    app.oauth = Some(oauth);
    let worker = outbox.clone();
    esp::spawn_worker(c"ml-outbox", 16 * 1024, false, move || loop {
        let now = miniframework::wall_ms().map_or(0, |ms| ms / 1000);
        if !worker.run_once(now) {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    })?;
    // Email from the build settings (the household .env's Brevo key), unless
    // the Household page set it up since.
    if let Some(key) = option_env!("MERITLEDGER_BREVO_API_KEY") {
        let setup = meritledger::outbound::MailSetup {
            api_key: key.into(),
            sender_email: option_env!("MERITLEDGER_MAIL_FROM").unwrap_or("").into(),
            built_in: true,
        };
        if let Err(e) = outbox.begin_mail_setup(setup) {
            log::warn!("email: {}", e.message);
        }
    }
    app.outbox = Some(outbox);
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
    // Calendar feeds and the Mastodon sign-in callback are the app's.
    config.app_paths = &["/cal", "/auth"];
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
    board.serve(site, move |site| {
        site.service.tick();
        screen.tick(&site.service);
    })
}

/// The board's screen and touch menu (`meritledger::screen`, Sprint 13).
///
/// Everything runs on the main task between housekeeping (the connection
/// loop is on the other core): touch is polled every 30 ms over I2C, the
/// menu logic is `screen::ui` (tested on the desktop), and the screen is
/// redrawn only when something visible changed, in twenty-two 5 KB strips
/// over SPI.
mod display {
    use esp_idf_svc::hal::gpio::{Gpio41, Gpio42, Gpio47, Gpio48, PinDriver, Pull};
    use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver, I2C0};
    use esp_idf_svc::hal::units::FromValueType;
    use meritledger::messages::ScreenSettings;
    use meritledger::screen::ui::{self, Action, Gesture, Person, Touch, Ui};
    use meritledger::screen::{canvas, Card, Context, Frame, STRIP};
    use meritledger::App;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    // The JD9853 strip driver (`components/display/ml_display.c`).
    extern "C" {
        fn ml_display_init() -> i32;
        fn ml_display_buffer() -> *mut u16;
        fn ml_display_orient(flip: bool) -> i32;
        fn ml_display_flush(top: i32, rows: i32, swap: bool) -> i32;
        fn ml_display_backlight(percent: i32) -> i32;
    }

    /// The panel: the only code that calls the C driver, so every `unsafe`
    /// for the screen is here. A `Panel` exists only after a successful
    /// init, and it alone holds the driver's DMA strip buffer.
    struct Panel {
        strip: &'static mut [u16],
    }

    static OPENED: AtomicBool = AtomicBool::new(false);

    impl Panel {
        fn open() -> Result<Panel, i32> {
            // Once per boot: a second `Panel` would alias the buffer.
            if OPENED.swap(true, Ordering::SeqCst) {
                return Err(-1);
            }
            // SAFETY: the driver's init, called once (just checked).
            let err = unsafe { ml_display_init() };
            if err != 0 {
                return Err(err);
            }
            // SAFETY: after a successful init, a DMA allocation of
            // W x STRIP pixels that the driver never frees.
            let ptr = unsafe { ml_display_buffer() };
            if ptr.is_null() {
                return Err(-2);
            }
            // SAFETY: non-null, aligned (heap_caps_malloc), the right
            // length, and only this one `Panel` ever touches it.
            let strip =
                unsafe { std::slice::from_raw_parts_mut(ptr, (canvas::W * STRIP) as usize) };
            Ok(Panel { strip })
        }

        fn set(&mut self, flip: bool, brightness: u32) {
            // SAFETY: the driver is initialized (a `Panel` exists).
            let (a, b) = unsafe {
                (
                    ml_display_orient(flip),
                    ml_display_backlight(brightness.min(100) as i32),
                )
            };
            if a != 0 || b != 0 {
                log::warn!("screen: settings failed ({a}, {b})");
            }
        }

        /// Draws `frame` strip by strip; big-endian pixels unless `swap` is
        /// false.
        fn show(&mut self, frame: &Frame, swap: bool) -> Result<(), i32> {
            frame.draw(self.strip, |top, rows, _| {
                // SAFETY: sends the strip just drawn into our buffer; the
                // call waits for its DMA, so the buffer is free again when
                // it returns, before the next strip is drawn.
                match unsafe { ml_display_flush(top, rows, swap) } {
                    0 => Ok(()),
                    e => Err(e),
                }
            })
        }
    }

    /// The AXS5106L touch controller (protocol from Waveshare's example, as
    /// ported in the MIT-licensed `axs5106l` crate): I2C 0x63, register
    /// 0x01 holds the touch count and 12-bit points.
    const TOUCH_ADDR: u8 = 0x63;
    /// INT is a pin read, so check it every tick (~10 ms): a quick tap's
    /// pulse is short. The bus is read only while INT or a finger says so.
    const POLL: Duration = Duration::from_millis(10);

    struct Chip {
        i2c: I2cDriver<'static>,
        /// Pulses low when the chip has a report. Reads happen only then (and
        /// while a finger is down), not on every tick as at first.
        int: PinDriver<'static, esp_idf_svc::hal::gpio::Input>,
        /// Raw frames logged so far (the first few, for bring-up).
        logged: u32,
        /// A finger is down: read frames until the chip's count is 0.
        /// INT only pulses once per report, so it wakes the reader but
        /// can't say when the finger lifts.
        down: bool,
    }

    impl Chip {
        /// A register read as the reference driver does it: the register
        /// address, a STOP, then the read. Not a repeated start, which this
        /// chip answers with zeros.
        fn reg(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), esp_idf_svc::sys::EspError> {
            self.i2c.write(TOUCH_ADDR, &[reg], 20)?;
            self.i2c.read(TOUCH_ADDR, buf, 20)
        }

        fn read(&mut self) -> Option<(i32, i32)> {
            if !self.down && self.int.is_high() {
                return None;
            }
            // gesture, count, XH, XL, YH, YL
            let mut data = [0u8; 6];
            self.reg(0x01, &mut data).ok()?;
            if data != [0; 6] && self.logged < 12 {
                self.logged += 1;
                log::info!("touch: frame {data:02x?}");
            }
            if data[1] == 0 || data[1] > 5 {
                self.down = false;
                return None;
            }
            self.down = true;
            let x = (i32::from(data[2] & 0x0f) << 8) | i32::from(data[3]);
            let y = (i32::from(data[4] & 0x0f) << 8) | i32::from(data[5]);
            Some((x, y))
        }
    }

    pub struct Screen {
        panel: Option<Panel>,
        chip: Option<Chip>,
        ui: Ui,
        touch: Touch,
        started: Instant,
        polled: Option<Instant>,
        pressed: bool,
        dirty: bool,
        minute: u64,
        applied: Option<ScreenSettings>,
        people: Vec<Person>,
        cards: usize,
        draws: u32,
        draw_ms: u128,
    }

    impl Screen {
        pub fn new(
            i2c: I2C0<'static>,
            sda: Gpio42<'static>,
            scl: Gpio41<'static>,
            int: Gpio48<'static>,
            rst: Gpio47<'static>,
        ) -> Screen {
            let panel = match Panel::open() {
                Ok(panel) => {
                    log::info!("screen: JD9853 ready");
                    Some(panel)
                }
                Err(err) => {
                    log::error!("screen: init failed ({err}); running without it");
                    None
                }
            };
            Screen {
                panel,
                chip: touch_chip(i2c, sda, scl, int, rst),
                ui: Ui::default(),
                touch: Touch::default(),
                started: Instant::now(),
                polled: None,
                pressed: false,
                dirty: true,
                minute: u64::MAX,
                applied: None,
                people: Vec::new(),
                cards: 1,
                draws: 0,
                draw_ms: 0,
            }
        }

        fn ms(&self) -> u64 {
            self.started.elapsed().as_millis() as u64
        }

        pub fn tick(&mut self, app: &App) {
            if self.panel.is_none() {
                return;
            }
            let now = self.ms();
            if self.polled.is_none_or(|t| t.elapsed() >= POLL) {
                self.polled = Some(Instant::now());
                self.poll_touch(app, now);
            }
            if self.ui.tick(now, self.cards) {
                self.dirty = true;
            }
            let wall = miniframework::wall_ms().map_or(0, |ms| ms / 1000);
            // The clock on the Today card changes once a minute.
            if wall / 60 != self.minute && !self.ui.in_menu() {
                self.minute = wall / 60;
                self.dirty = true;
            }
            if self.dirty {
                self.dirty = false;
                self.draw(app, wall);
            }
        }

        fn poll_touch(&mut self, app: &App, now: u64) {
            let Some(chip) = self.chip.as_mut() else {
                return;
            };
            let raw = chip.read();
            let settings = self.applied.clone().unwrap_or_default();
            let at =
                raw.map(|(x, y)| ui::landscape(x, y, settings.touch_flip_x, settings.touch_flip_y));
            // Note where each press lands (for calibrating on the admin page).
            if let (Some(r), Some(s), false) = (raw, at, self.pressed) {
                app.note_touch(r, s);
            }
            self.pressed = raw.is_some();
            let Some(gesture) = self.touch.sample(at, now) else {
                return;
            };
            if matches!(gesture, Gesture::LongPress) || self.ui.in_menu() {
                self.people = app.board_people();
            }
            let action = self.ui.gesture(gesture, &self.people, self.cards, now);
            self.dirty = true;
            match action {
                Some(Action::Record { person, kind }) => {
                    match app.record_from_board(person, kind) {
                        Ok(entry) => {
                            let name = self
                                .people
                                .iter()
                                .find(|p| p.id == person)
                                .map_or(String::new(), |p| p.name.clone());
                            self.ui.recorded(
                                entry,
                                format!("{} for {name}", app.kind_name(kind)),
                                now,
                            );
                        }
                        Err(e) => self.ui.notice(e.message, now),
                    }
                }
                Some(Action::Undo { entry }) => {
                    let text = match app.undo_from_board(entry) {
                        Ok(()) => "Undone".to_string(),
                        Err(e) => e.message,
                    };
                    self.ui.notice(text, now);
                }
                Some(Action::Call) => {
                    let text = match app.call_from_board() {
                        Ok(n) => format!("Called {n} to practice"),
                        Err(e) => e.message,
                    };
                    self.ui.notice(text, now);
                }
                None => {}
            }
        }

        fn draw(&mut self, app: &App, wall: u64) {
            let started = Instant::now();
            let (cards, settings) = if wall == 0 {
                (Vec::new(), app.screen(0).1)
            } else {
                app.screen(wall)
            };
            let Some(panel) = self.panel.as_mut() else {
                return;
            };
            if self.applied.as_ref() != Some(&settings) {
                panel.set(settings.flip, settings.brightness);
                self.applied = Some(settings.clone());
            }
            let cards = if cards.is_empty() {
                vec![Card::Message {
                    title: "Merit Ledger".into(),
                    body: "Joining Wi-Fi and setting the clock".into(),
                    button: String::new(),
                }]
            } else {
                cards
            };
            self.cards = cards.len();
            let (card, page, pages) = self.ui.view(&cards, &self.people);
            let frame = Frame::new(
                card,
                Context {
                    name: "meritledger.local".into(),
                    page,
                    pages,
                },
            );
            // Big-endian pixels unless the `native` setting says otherwise.
            let Some(panel) = self.panel.as_mut() else {
                return;
            };
            if let Err(e) = panel.show(&frame, !settings.native) {
                log::warn!("screen: flush failed ({e})");
            }
            self.draws += 1;
            self.draw_ms += started.elapsed().as_millis();
            if self.draws.is_multiple_of(100) {
                log::info!(
                    "screen: {} redraws, {} ms each on average",
                    self.draws,
                    self.draw_ms / 100
                );
                self.draw_ms = 0;
            }
        }
    }

    /// Brings up the touch controller: reset on GPIO 47, interrupt on 48
    /// (CircuitPython's naming; the factory log's the other way round, and
    /// with it the chip answered but INT never moved, on the board
    /// 2026-10-19). If the chip doesn't answer, the screen works untouched.
    fn touch_chip(
        i2c: I2C0<'static>,
        sda: Gpio42<'static>,
        scl: Gpio41<'static>,
        int: Gpio48<'static>,
        rst: Gpio47<'static>,
    ) -> Option<Chip> {
        let started = || -> Result<Chip, esp_idf_svc::sys::EspError> {
            let int = PinDriver::input(int, Pull::Up)?;
            let mut reset = PinDriver::output(rst)?;
            reset.set_high()?;
            let config = I2cConfig::new().baudrate(400.kHz().into());
            let int_low = int.is_low();
            let mut chip = Chip {
                i2c: I2cDriver::new(i2c, sda, scl, &config)?,
                int,
                logged: 0,
                down: false,
            };
            // The reference driver's reset (mydazy/esp_lcd_touch_axs5106l,
            // Apache-2.0): a soft reset command, then a 20 ms pulse, 50 ms
            // to settle.
            let _ = chip
                .i2c
                .write(TOUCH_ADDR, &[0xf0, 0xb3, 0x55, 0xaa, 0x34, 0x01], 20);
            reset.set_low()?;
            std::thread::sleep(Duration::from_millis(20));
            reset.set_high()?;
            std::thread::sleep(Duration::from_millis(50));
            // Held high for the life of the firmware.
            std::mem::forget(reset);
            // The ID is all zeros or all ones until the chip is up.
            let mut id = [0u8; 3];
            for _ in 0..5 {
                if chip.reg(0x08, &mut id).is_ok() && id != [0; 3] && id != [0xff; 3] {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let mut version = [0u8; 2];
            let _ = chip.reg(0x05, &mut version);
            log::info!(
                "touch: AXS5106L id {:02x}{:02x}{:02x}, firmware {}, INT {}",
                id[0],
                id[1],
                id[2],
                u16::from_be_bytes(version),
                if int_low { "low" } else { "high" }
            );
            if id == [0; 3] || id == [0xff; 3] {
                log::warn!("touch: the chip ACKs but its ID is blank; touches may not report");
            }
            Ok(chip)
        };
        match started() {
            Ok(chip) => Some(chip),
            Err(e) => {
                log::warn!("touch: no AXS5106L at 0x{TOUCH_ADDR:02x} ({e})");
                None
            }
        }
    }
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
