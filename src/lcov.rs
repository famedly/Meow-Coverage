//! Helpers for handling code coverage report in the `lcov` format
use std::path::Path;

use lcov::{report::ParseError, Record, Report};

/// A per-file "coverage report" (contains only unhit lines)
#[derive(Debug, Clone)]
pub struct LcovFileCoverage {
	/// File name
	pub filename: String,
	/// Untested lines
	pub lines: Vec<u32>,
}

/// Wrapper for operations on a coverage reports
#[derive(Debug)]
pub struct LcovWrapper(Vec<Record>);

impl LcovWrapper {
	/// Build a new [LcovWrapper]
	pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, ParseError> {
		Report::from_file(file_path)
			.map(Report::into_records)
			.map(Iterator::collect::<Vec<_>>)
			.map(Self)
	}

	/// Calculate the percentage coverage
	#[must_use]
	pub fn percentage(&self) -> f64 {
		let (lines_hit, lines_found) =
			self.0.iter().fold((0, 0), |(line_hits, lines_found), record| match record {
				Record::LinesHit { hit } => (line_hits + u64::from(*hit), lines_found),
				Record::LinesFound { found } => (line_hits, lines_found + u64::from(*found)),
				_ => (line_hits, lines_found),
			});

		(lines_hit as f64 / lines_found as f64) * 100.0
	}

	/// Diff the percentages of a newer coverage file with the current one
	#[must_use]
	pub fn percentage_difference(&self, new_lcov: &Self) -> f64 {
		new_lcov.percentage() - self.percentage()
	}

	/// Group coverage data by file
	#[must_use]
	pub fn group_data(&self) -> Vec<LcovFileCoverage> {
		let mut files = Vec::new();

		for record in &self.0 {
			match record {
				Record::SourceFile { path } => files.push(LcovFileCoverage {
					filename: path.to_string_lossy().to_string(),
					lines: Vec::new(),
				}),
				Record::LineData { line, count, .. } => {
					if *count == 0 {
						if let Some(last) = files.last_mut() {
							last.lines.push(*line);
						}
					}
				}
				_ => {}
			}
		}

		files
	}
}
