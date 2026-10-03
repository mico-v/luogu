use crate::auth;
use crate::cli::LoginArgs;
use crate::net;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::fs;
use std::io::{self, IsTerminal, Write};

pub fn run(args: LoginArgs) -> Result<()> {
    if args.cookie.is_some() || args.cookie_file.is_some() {
        return import_cookie(args);
    }

    let username = match args.username {
        Some(value) if !value.trim().is_empty() => value,
        _ => prompt("Username: ")?,
    };
    if username.trim().is_empty() {
        return Err(anyhow!("username cannot be empty"));
    }

    let password = match args.password {
        Some(value) if !value.is_empty() => value,
        _ => rpassword::prompt_password("Password: ").context("read password")?,
    };
    if password.is_empty() {
        return Err(anyhow!("password cannot be empty"));
    }

    let session = net::LoginSession::new()?;
    let mut captcha_file = None;
    let captcha = match args.captcha {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            let image = session.fetch_captcha()?;
            if io::stdout().is_terminal() {
                print_captcha(&image)?;
            } else {
                let captcha_path = auth::captcha_path()?;
                fs::write(&captcha_path, image)
                    .with_context(|| format!("write {}", captcha_path.display()))?;
                println!(
                    "{} Captcha saved to {}. Open it and enter the text below.",
                    "ℹ".cyan(),
                    captcha_path.display().to_string().cyan()
                );
                captcha_file = Some(captcha_path);
            }
            prompt("Captcha: ")?
        }
    };
    if captcha.trim().is_empty() {
        return Err(anyhow!("captcha cannot be empty"));
    }

    let (response, cookie) = session.password_login(username.trim(), &password, captcha.trim())?;
    if response.locked {
        return Err(anyhow!(
            "account is locked; complete the verification in a browser and try again"
        ));
    }

    let uid = auth::uid_from_cookie_header(&cookie);
    auth::save_session(&cookie, Some(&response.username), uid)?;
    if let Some(captcha_path) = captcha_file {
        fs::remove_file(captcha_path).ok();
    }

    println!(
        "{} Logged in as {} (UID: {}).",
        "✓".green(),
        response.username.cyan().bold(),
        uid.map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
            .cyan()
    );
    println!(
        "{} Session saved to {}.",
        "ℹ".cyan(),
        auth::session_path().display().to_string().cyan()
    );
    Ok(())
}

pub fn logout() -> Result<()> {
    if auth::clear_session()? {
        println!("{} Local Luogu session removed.", "✓".green());
    } else {
        println!("{} No local Luogu session found.", "ℹ".cyan());
    }
    Ok(())
}

fn print_captcha(bytes: &[u8]) -> Result<()> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
        .context("decode captcha image")?
        .to_rgb8();
    let max_width = terminal_width().saturating_sub(2).clamp(32, 120);
    print!("{}", render_captcha(&image, max_width));
    io::stdout().flush().context("flush captcha image")
}

fn terminal_width() -> u32 {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(80)
        .clamp(32, 120)
}

fn render_captcha(image: &image::RgbImage, max_width: u32) -> String {
    let (width, height) = image.dimensions();
    let scale = (max_width as f32 / width as f32).min(1.0);
    let target_width = (width as f32 * scale).round().max(1.0) as u32;
    let target_height = (height as f32 * scale).round().max(2.0) as u32;
    let resized = image::imageops::resize(
        image,
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    let mut output = String::new();
    for y in (0..target_height).step_by(2) {
        for x in 0..target_width {
            let top = resized.get_pixel(x, y).0;
            let bottom = if y + 1 < target_height {
                resized.get_pixel(x, y + 1).0
            } else {
                [0, 0, 0]
            };
            output.push_str(&format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]
            ));
        }
        output.push_str("\x1b[0m\n");
    }
    output
}

fn import_cookie(args: LoginArgs) -> Result<()> {
    if args.cookie.is_some() && args.cookie_file.is_some() {
        return Err(anyhow!("use either --cookie or --cookie-file, not both"));
    }

    let raw = if let Some(cookie) = args.cookie {
        cookie
    } else {
        let path = args.cookie_file.expect("cookie file checked above");
        fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?
    };
    let cookie = auth::normalize_cookie_header(&raw)?;
    let user = net::validate_session_cookie(&cookie)?;
    auth::save_session(&cookie, Some(&user.name), Some(user.uid))?;

    println!(
        "{} Imported session for {} (UID: {}).",
        "✓".green(),
        user.name.cyan().bold(),
        user.uid.to_string().cyan()
    );
    Ok(())
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush().context("flush prompt")?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .context("read terminal input")?;
    Ok(value.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn renders_captcha_with_truecolor_blocks() {
        let image = image::RgbImage::from_fn(4, 4, |x, y| Rgb([(x * 20) as u8, (y * 20) as u8, 0]));
        let rendered = render_captcha(&image, 4);

        assert!(rendered.contains("\x1b[38;2;"));
        assert!(rendered.contains("\x1b[48;2;"));
        assert_eq!(rendered.lines().count(), 2);
    }
}
