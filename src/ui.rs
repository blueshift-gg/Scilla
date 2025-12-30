use {
    console::style,
    indicatif::{ProgressBar, ProgressStyle},
};

pub async fn show_spinner<F, T>(message: &str, fut: F) -> anyhow::Result<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .expect("Failed to create progress bar template - this is a bug in the template string")
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner.set_message(message.to_string());

    let result = fut.await;
    spinner.finish_with_message("✅ Done");

    result
}

pub fn print_error(message: impl std::fmt::Display) {
    println!("\n{}\n", style(message).red().bold());
}
