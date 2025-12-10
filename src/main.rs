mod claude;

use claude::{ClaudeAiOauth, ClaudeCredentials};
use image::GenericImageView;
use ksni::{TrayMethods, menu::*};
use std::sync::LazyLock;

// Loading icon image that is used in the tray
static CLAUDE_ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
    let img = image::load_from_memory_with_format(
        include_bytes!("./assets/claude-icon.png"),
        image::ImageFormat::Png,
    )
    .expect("valid image");

    let (width, height) = img.dimensions();
    let mut data = img.into_rgba8().into_vec();

    for pixel in data.chunks_exact_mut(4) {
        pixel.rotate_right(1)
    }

    ksni::Icon {
        width: width as i32,
        height: height as i32,
        data,
    }
});

// Tray variables to handle authentication and usage tracking
#[derive(Debug)]
struct AppTray {
    // Indicates whether the user is logged in
    login: bool,
    // Credentials for making requests to the Claude API. To get this
    // information is necessary to login using claude console command
    access_token: claude::ClaudeCredentials,
    // Variables to track usage
    five_hour_usage: f32,
    seven_day_usage: f32,
}

// Options to show in the tray menu application
impl ksni::Tray for AppTray {
    // Identifier for the tray
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }
    // Custome icon for the tray
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![CLAUDE_ICON.clone()]
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        println!("Usuario hizo clic en el tray!");
        // Aquí puedes ejecutar tu lógica
    }
    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        // El menú se muestra automáticamente
        println!("Mostrando menú");
    }
    // Title for the tray
    fn title(&self) -> String {
        "Claude Tray".into()
    }
    // Menu items for the tray
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            // Login option in case user is not logged in
            StandardItem {
                label: "Iniciar sesión".into(),
                // If login is false, show login option
                visible: !self.login,
                activate: Box::new(|this: &mut Self| match claude::login() {
                    Ok(access_token) => {
                        this.login = true;
                        this.access_token = access_token;
                    }
                    Err(err) => eprintln!("Error logging in: {}", err),
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Plan usage limits\nCurrent session ({}/100)",
                    self.five_hour_usage
                ),
                visible: self.login,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Weekly usage limits\nAll models ({}/100)",
                    self.seven_day_usage
                ),
                visible: self.login,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Option to open ClaudeIA using the browser
            StandardItem {
                label: "Open Claude".into(),
                activate: Box::new(|_| {
                    webbrowser::open("https://claude.ai/new")
                        .expect("Error opening Claude on the browser")
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Option to exit the application
            StandardItem {
                label: "Exit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut login = false;

    // Try to load credentials from file. In case of error, assume user is
    // not logged in claude account and will show login option in the tray menu
    let access_token: claude::ClaudeCredentials = match claude::get_credentials() {
        Ok(credentials) => {
            login = true;
            credentials
        }
        Err(err) => {
            println!("Error getting credentials: {}", err);

            // Set default credentials values. Maybe
            // use Option::None instead of an empty object?
            ClaudeCredentials {
                claude_ai_oauth: ClaudeAiOauth {
                    access_token: "".into(),
                    refresh_token: "".into(),
                    expires_at: 0,
                    scopes: vec![],
                    subscription_type: "".into(),
                    rate_limit_tier: "".into(),
                },
            }
        }
    };

    let mut five_hour_usage: f32 = 0.0;
    let mut seven_day_usage: f32 = 0.0;

    if !access_token.claude_ai_oauth.access_token.is_empty() {
        // If credentials are valid, try to get usage
        match claude::get_usage(&access_token.claude_ai_oauth.access_token).await {
            Ok(usage) => {
                five_hour_usage = usage.five_hour.utilization;
                seven_day_usage = usage.seven_day.utilization;
            }
            Err(err) => {
                println!("Error getting usage: {}", err);
            }
        }
    }

    let tray = AppTray {
        login,
        access_token,
        five_hour_usage,
        seven_day_usage,
    };

    // let handle =
    tray.spawn().await.unwrap();

    // We can modify the tray
    // handle
    //     .update(|tray: &mut UsageStatus| tray.checked = true)
    //     .await;
    // Run forever
    std::future::pending().await
}
