//! This module contains shared definitions and helpers for tracking coverage
//! and constructing centralised visualisations

mod management;
mod models;
mod visualisation;

use std::{fmt::Display, path::Path};

pub use management::*;
pub use models::*;
use octocrab::models::repos::CommitAuthor;

use crate::{github_api::get_file_sha, MeowCoverageError};

/// Constant for the `records` branch
pub const RECORDS_BRANCH: &str = "records";

/// Builds the [CommitAuthor] used for operations on the centralised coverage
/// tracking repository
pub fn author() -> CommitAuthor {
	CommitAuthor {
		name: String::from("Meow! Coverage"),
		email: String::from("e.mansbridge+meow-coverage@famedly.de"),
	}
}

/// Make a report path by `owner`, `repo`, and `branch`
pub fn make_report_path(owner: &str, repo: &str, branch: &str) -> String {
	format!("{}/{}/{}.meowcov.json", owner, repo, branch)
}

/// Turn an f64 percentage into a u16 percentage
fn make_percent(percentage: f64) -> i16 {
	(percentage.clamp(-100_f64, 100_f64) * 100_f64).round().clamp(-10000_f64, 10000_f64) as i16
}

/// Rebuild the visualisation for a single project (and the README)
pub async fn rebuild(
	records: &Path,
	coverage_repo: &str,
	target_repo: &str,
	branch: &str,
) -> Result<(), MeowCoverageError> {
	let branch = branch.trim_start_matches("refs/heads/");
	let (coverage_repo_owner, coverage_repo) =
		coverage_repo.split_once('/').ok_or(MeowCoverageError::RepoNameMissingSlash)?;
	let (target_repo_owner, target_repo) =
		target_repo.split_once('/').ok_or(MeowCoverageError::RepoNameMissingSlash)?;

	let record_collection: BranchCoverageRecordCollection = {
		let mut path = records.to_owned();
		path.push(target_repo_owner);
		path.push(target_repo);
		path.push(format!("{}.meowcov.json", branch));

		serde_json::from_reader(std::fs::File::open(path)?)?
	};

	let Some(coverage_report) = visualisation::build_coverage_report(target_repo_owner, target_repo, branch, &record_collection) else {
		return Ok(())
	};
	let readme = visualisation::build_readme(records, coverage_repo_owner, coverage_repo)?;

	let report_path = format!("reports/{}/{}/{}.md", target_repo_owner, target_repo, branch);

	let readme_sha = get_file_sha(
		coverage_repo_owner,
		coverage_repo,
		octocrab::params::repos::Reference::Branch(String::from("main")),
		"README.md",
	)
	.await?;
	let other_sha = get_file_sha(
		coverage_repo_owner,
		coverage_repo,
		octocrab::params::repos::Reference::Branch(String::from("main")),
		report_path.as_str(),
	)
	.await
	.ok();

	octocrab::instance()
		.repos(coverage_repo_owner, coverage_repo)
		.update_file("README.md", "Update README", readme.as_bytes(), readme_sha)
		.branch("main")
		.author(author())
		.commiter(author())
		.send()
		.await?;
	match other_sha {
		Some(sha) => {
			octocrab::instance()
				.repos(coverage_repo_owner, coverage_repo)
				.update_file(
					report_path.as_str(),
					&format!("Update report for {}/{}/{}", target_repo_owner, target_repo, branch),
					coverage_report.as_bytes(),
					sha,
				)
				.branch("main")
				.author(author())
				.commiter(author())
				.send()
				.await?;
		}
		None => {
			octocrab::instance()
				.repos(coverage_repo_owner, coverage_repo)
				.create_file(
					report_path.as_str(),
					&format!("Create report for {}/{}/{}", target_repo_owner, target_repo, branch),
					coverage_report.as_bytes(),
				)
				.branch("main")
				.author(author())
				.commiter(author())
				.send()
				.await?;
		}
	}

	Ok(())
}

/// Wrapper for displaying an i16 percent correctly
struct PercentWrapper(i16);

impl Display for PercentWrapper {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_fmt(format_args!("{:.2}", f64::from(self.0) / 100_f64))
	}
}
