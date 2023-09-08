//! Visually check coverage of a local Rust project

use std::process::Command;

use clap::Parser;
use meow_coverage_shared::{lcov::report::ParseError, path_split, LcovWrapper};
use owo_colors::OwoColorize;

/// cargo-meow-coverage
#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CliArgsWrapper {
	/// license-check
	MeowCoverage(CliArgs),
}

impl CliArgsWrapper {
	/// Return the inner [CliArgs]
	pub fn into_inner(self) -> CliArgs {
		match self {
			Self::MeowCoverage(args) => args,
		}
	}
}

/// cargo-meow-coverage
#[derive(Debug, clap::Args)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
	/// Only display a summary of the report
	#[clap(long, action)]
	pub only_summary: bool,

	/// Print a list of files seperatly
	#[clap(long, action)]
	pub list_files: bool,
}

/// Error collection for cargo-meow-coverage
#[derive(Debug, thiserror::Error)]
pub enum CargoMeowCoverageError {
	/// LCOV parsing error ([ParseError])
	#[error("Lcov Parsing Failed: {0}")]
	LcovParse(#[from] ParseError),
	/// IO error whilst running `cargo llvm-cov` subcommand
	#[error("IO error whilst running cargo: {0}")]
	CommandIo(std::io::Error),
	/// `cargo llvm-cov` returned non-zero exit code
	#[error(r#""cargo llvm-cov" exited with {0} (try running "cargo llvm-cov --lcov" to debug the error)"#)]
	CoverageNonZeroExit(i32),
	/// IO error whilst reading source file
	#[error("IO error whilst reading source file: {0}")]
	SourceReadError(std::io::Error),
}

/// File coverage for local repositories
#[derive(Debug)]
pub struct LocalFileCoverage {
	/// Collection of unclumped lines
	pub raw_lines: Vec<u32>,
	/// Percentage coverage
	pub percentage: f64,
	/// File Path
	pub path: String,
}

/// Report returned by [local_coverage]
#[derive(Debug)]
pub struct LocalCoverageReport {
	/// List of paths to all files with 100% coverage
	pub tested_files: Vec<String>,
	/// List of reports of files that do not have 100% coverage
	pub untested_files: Vec<LocalFileCoverage>,
	/// Coverage percentage
	pub percentage: f64,
	/// Total file count
	pub file_count: usize,
}

/// Build a local coverage report for a local project
fn local_coverage(report: &[u8], source_prefix: &str) -> Result<LocalCoverageReport, ParseError> {
	let lcov = LcovWrapper::with_report(report)?;

	let file_count = lcov.file_count();
	let percentage = lcov.percentage();
	let lcov_data = lcov.group_data();

	let tested_files = lcov_data
		.iter()
		.filter_map(|coverage| {
			if !coverage.lines.is_empty() {
				return None;
			}

			let path = path_split(coverage.filename.as_str(), source_prefix);
			Some(path)
		})
		.collect::<Vec<_>>();

	let untested_files = lcov_data
		.into_iter()
		.filter_map(|coverage| {
			if coverage.lines.is_empty() {
				return None;
			}

			let path = path_split(coverage.filename.as_str(), source_prefix);
			Some(LocalFileCoverage {
				raw_lines: coverage.lines,
				path,
				percentage: coverage.percentage,
			})
		})
		.collect::<Vec<_>>();

	Ok(LocalCoverageReport { tested_files, untested_files, percentage, file_count })
}

/// Counts the amount of digits the number will have when represented in base 10
fn digit_count(mut line: u32) -> u32 {
	let mut digits = 0;

	while line != 0 {
		digits += 1;
		line /= 10;
	}

	digits
}

/// Print a source code line
#[allow(clippy::print_stdout)]
fn print_line(largest_digit_count: u32, line: u32, content: &str, tested: bool) {
	let digit_count = digit_count(line);

	for _ in 0..(largest_digit_count + 1 - digit_count) {
		print!(" ");
	}

	if tested {
		print!("{}{}", line.blue(), " | ".blue());
	} else {
		print!("{}{}", line.red(), " | ".red());
	}

	println!("{}", content);
}

/// Wrapped main function for capturing the error to display properly
#[allow(clippy::print_stdout)]
fn real_main() -> Result<(), CargoMeowCoverageError> {
	let args = CliArgsWrapper::parse().into_inner();

	let llvmcov_output = Command::new("cargo")
		.arg("llvm-cov")
		.arg("--lcov")
		.output()
		.map_err(CargoMeowCoverageError::CommandIo)?;

	if !llvmcov_output.status.success() {
		return Err(CargoMeowCoverageError::CoverageNonZeroExit(
			llvmcov_output.status.code().unwrap_or_default(),
		));
	}

	let report = local_coverage(&llvmcov_output.stdout, "src/")?;

	if !args.only_summary {
		for file in &report.untested_files {
			let raw_source = std::fs::read_to_string(file.path.as_str())
				.map_err(CargoMeowCoverageError::SourceReadError)?;
			let line_contents = raw_source.split('\n').collect::<Vec<_>>();

			let largest_line = file.raw_lines.last().copied().unwrap_or_default();

			let largest_digit_count = digit_count(largest_line);
			println!(
				"{} {} {} {}",
				"Found".red().bold(),
				file.raw_lines.len(),
				"untested lines in".red().bold(),
				file.path
			);

			let mut last_line = 0;
			for &line in &file.raw_lines {
				if last_line != 0 && last_line + 5 >= line {
					for line in (last_line + 1)..line {
						print_line(
							largest_digit_count,
							line,
							line_contents[(line - 1) as usize],
							true,
						);
					}
				} else if last_line == 0 || last_line + 1 != line {
					println!("{} {}:{}", "-->".blue(), file.path, line);
				}

				print_line(largest_digit_count, line, line_contents[(line - 1) as usize], false);
				last_line = line;
			}

			println!();
		}
	}

	if args.list_files {
		println!(
			"{}",
			format!("Fully Tested Files ({})", report.tested_files.len())
				.green()
				.bold()
				.underline()
		);

		for file in &report.tested_files {
			println!("{}", file);
		}

		println!(
			"\n{}",
			format!("Untested/Partially Tested Files ({})", report.untested_files.len())
				.red()
				.bold()
				.underline()
		);

		for file in &report.untested_files {
			println!("{} ({:.2}%)", file.path, file.percentage);
		}

		println!();
	}

	// Summary
	println!(
		"{} {}/{}\n{} {:.2}%",
		"Fully Covered Files:".bold(),
		report.tested_files.len(),
		report.file_count,
		"Coverage Percentage:".bold(),
		report.percentage
	);

	Ok(())
}

#[allow(clippy::print_stderr)]
fn main() -> Result<(), ()> {
	real_main().map_err(|why| {
		eprintln!("{}", why);
	})
}
