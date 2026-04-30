use colored::Colorize;

#[must_use]
pub fn action(label: &str) -> String {
    label.bright_yellow().bold().to_string()
}

#[must_use]
pub fn success(label: &str) -> String {
    label.bright_green().bold().to_string()
}

#[must_use]
pub fn warning(label: &str) -> String {
    label.yellow().bold().to_string()
}

#[must_use]
pub fn path(label: &impl ToString) -> String {
    label.to_string().bright_green().to_string()
}

#[must_use]
pub fn command(label: &str) -> String {
    label.bright_yellow().to_string()
}
