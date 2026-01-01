use {
    console::style,
    indicatif::{ProgressBar, ProgressStyle},
};

pub async fn show_spinner<F, T>(message: &str, fut: F)
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner.set_message(message.to_string());

    let result = fut.await;

    match &result {
        Ok(_) => spinner.finish_with_message("✅ Done"),
        Err(e) => {
            spinner.finish_with_message(format!("{}", style(format!("Error : {}", e)).red().bold()))
        }
    }
}

pub fn print_error(message: impl std::fmt::Display) {
    println!("{}", style(message).red().bold());
}
