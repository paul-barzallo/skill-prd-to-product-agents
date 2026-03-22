use anyhow::Result;
use colored::Colorize;
use std::error::Error;
use std::fmt;

pub mod workspace;
pub mod prompts;
pub mod agents;
pub mod governance;
pub mod readiness;
pub mod models;
pub mod ci;

#[derive(Debug)]
pub struct ValidationFailure {
	message: String,
}

impl ValidationFailure {
	pub fn new(message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
		}
	}
}

impl fmt::Display for ValidationFailure {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.message)
	}
}

impl Error for ValidationFailure {}

pub fn validation_failure(message: impl Into<String>) -> anyhow::Error {
	ValidationFailure::new(message).into()
}

pub fn is_validation_failure(error: &anyhow::Error) -> bool {
	error.downcast_ref::<ValidationFailure>().is_some()
}

pub fn log_validation_summary(name: &str, errors: u32, warnings: u32, checked: Option<u32>) {
	match (errors > 0, warnings > 0, checked) {
		(true, _, Some(checked)) => {
			tracing::error!(validation = %name, checked, errors, warnings, "validation failed");
		}
		(true, _, None) => {
			tracing::error!(validation = %name, errors, warnings, "validation failed");
		}
		(false, true, Some(checked)) => {
			tracing::warn!(validation = %name, checked, warnings, "validation passed with warnings");
		}
		(false, true, None) => {
			tracing::warn!(validation = %name, warnings, "validation passed with warnings");
		}
		(false, false, Some(checked)) => {
			tracing::info!(validation = %name, checked, "validation passed");
		}
		(false, false, None) => {
			tracing::info!(validation = %name, "validation passed");
		}
	}
}

pub fn finalize_validation(
	name: &str,
	errors: u32,
	warnings: u32,
	checked: Option<u32>,
	success_message: &str,
) -> Result<()> {
	log_validation_summary(name, errors, warnings, checked);

	if errors > 0 {
		eprintln!(
			"{} {errors} error(s), {warnings} warning(s)",
			"FAILED:".red().bold()
		);
		return Err(validation_failure(format!(
			"{name} validation failed with {errors} error(s) and {warnings} warning(s)"
		)));
	} else if warnings > 0 {
		println!(
			"{} 0 errors, {warnings} warning(s)",
			"PASSED (with warnings):".yellow().bold()
		);
	} else {
		println!("{} {success_message}", "PASSED:".green().bold());
	}

	Ok(())
}

pub fn extract_frontmatter(content: &str) -> Option<String> {
	if !content.starts_with("---") {
		return None;
	}

	let rest = &content[3..];
	let end = rest.find("---")?;
	Some(rest[..end].to_string())
}
