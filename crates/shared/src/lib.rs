//! Meow

mod helpers;

use std::path::Path;

pub use helpers::*;
pub use lcov;
use lcov::{report::ParseError, Reader, Record, Report};
#[cfg(feature = "patch")]
pub use patch;

/// A per-file "coverage report" (contains only unhit lines)
#[derive(Debug, Clone)]
pub struct LcovFileCoverage {
	/// File name
	pub filename: String,
	/// Percentage file coverage
	pub percentage: f64,
	/// Untested lines
	pub lines: Vec<u32>,
}

/// Wrapper for operations on a coverage reports
#[derive(Debug)]
pub struct LcovWrapper(Vec<Record>);

impl LcovWrapper {
	/// Build a new [LcovWrapper] from a report
	pub fn with_report(report: &[u8]) -> Result<Self, ParseError> {
		Report::from_reader(Reader::new(report))
			.map(Report::into_records)
			.map(Iterator::collect::<Vec<_>>)
			.map(Self)
	}

	/// Build a new [LcovWrapper]
	pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, ParseError> {
		Report::from_file(file_path)
			.map(Report::into_records)
			.map(Iterator::collect::<Vec<_>>)
			.map(Self)
	}

	/// Calculate the amount of files present in the report
	#[must_use]
	pub fn file_count(&self) -> usize {
		self.0.iter().fold(0, |count, record| match record {
			Record::SourceFile { .. } => count + 1,
			_ => count,
		})
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
		let mut lines_hit = None;
		let mut lines_found = None;

		for record in &self.0 {
			match record {
				Record::SourceFile { path } => {
					lines_hit = None;
					lines_found = None;

					files.push(LcovFileCoverage {
						filename: path.to_string_lossy().to_string(),
						percentage: 0_f64,
						lines: Vec::new(),
					});
				}
				Record::LineData { line, count, .. } => {
					if *count == 0 {
						if let Some(last) = files.last_mut() {
							last.lines.push(*line);
						}
					}
				}
				Record::LinesHit { hit } => {
					lines_hit = Some(*hit);

					if let Some(found) = lines_found {
						let percentage = f64::from(*hit) / f64::from(found);
						if let Some(last) = files.last_mut() {
							last.percentage = percentage;
						}

						lines_found = None;
						lines_hit = None;
					}
				}
				Record::LinesFound { found } => {
					lines_found = Some(*found);

					if let Some(hit) = lines_hit {
						let percentage = f64::from(hit) / f64::from(*found);
						if let Some(last) = files.last_mut() {
							last.percentage = percentage;
						}

						lines_found = None;
						lines_hit = None;
					}
				}
				_ => {}
			}
		}

		for file in &mut files {
			file.lines.sort();
		}

		files
	}
}
