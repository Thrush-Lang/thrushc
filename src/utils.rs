use {
    super::logging,
    std::process,
    stylic::{style, Color, Stylize},
};

#[inline]
pub fn extract_file_name(path: &str) -> String {
    path.split('/').next().unwrap().to_string()
}

pub fn notify_posible_issue(msg: &str) {
    println!("\n{}\n", style("POSSIBLE ISSUE").bold().bright_red());
    logging::log(logging::LogType::ERROR, msg);
    println!("{}", style("^".repeat(msg.len() + 5)).bold().bright_red());

    println!(
        "If the error is verified please report it: {}",
        style("https://github.com/Thrush-Lang/Thrush/issues/new").fg(Color::Rgb(141, 141, 142))
    );

    process::exit(1)
}
