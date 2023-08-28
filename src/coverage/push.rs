//! Module contains definitions for coverage operations on individual commits

use std::{borrow::Cow, collections::HashMap};

use hyper::StatusCode;
use octocrab::params::repos::Reference;
use sha2::{Digest, Sha256};

use super::{helpers::path_split, html::build_push_summary, lcov::LcovWrapper};
use crate::{
	github_api::get_file_sha,
	tracking::{
		author, make_report_path, BranchCoverageRecordCollection, FileCoverageRecord, Team,
		RECORDS_BRANCH,
	},
	MeowCoverageError,
};

/// File coverage wrapper for commits
#[derive(Debug)]
pub struct PushFileCoverageWrapper {
	/// File Git SHA
	pub sha: String,
	/// Collection of unclumped lines
	pub raw_lines: Vec<u32>,
	/// Percentage coverage
	pub percentage: f64,
	/// File Path
	pub realpath: String,
}

/// Generates a report for a commit
#[allow(clippy::too_many_lines)]
pub async fn generate_push_coverage_report(
	lcov_path: &str,
	repo_name: &str,
	source_prefix: &str,
	commit_sha: &str,
	coverage_colllecton_info: Option<(&str, &str, Team)>,
) -> Result<(), MeowCoverageError> {
	let lcov = LcovWrapper::new(lcov_path)?;

	let (owner, repo) = repo_name.split_once('/').ok_or(MeowCoverageError::RepoNameMissingSlash)?;

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

	let untested_changes = lcov_data
		.into_iter()
		.filter_map(|coverage| {
			if coverage.lines.is_empty() {
				return None;
			}

			let path = path_split(coverage.filename.as_str(), source_prefix);
			Some(PushFileCoverageWrapper {
				raw_lines: coverage.lines,
				sha: {
					let mut hasher = Sha256::new();
					hasher.update(path.as_str());
					hex::encode(hasher.finalize())
				},
				realpath: path,
				percentage: coverage.percentage,
			})
		})
		.collect::<Vec<_>>();

	octocrab::instance()
		.commits(owner, repo)
		.create_comment(
			commit_sha,
			format!(
				"<h3>Meow! Coverage</h3>Total: {:.2}%\n\n{}",
				lcov.percentage(),
				match untested_changes.is_empty() {
					true => Cow::Borrowed("🐾 All changes are tested! 🐾"),
					false =>
						Cow::Owned(build_push_summary(owner, repo, commit_sha, &untested_changes)),
				}
			),
		)
		.send()
		.await?;

	if let Some((branch, coverage_repo, team)) = coverage_colllecton_info {
		let report_path = make_report_path(owner, repo, branch);
		let (coverage_owner, coverage_repo) =
			coverage_repo.split_once('/').ok_or(MeowCoverageError::RepoNameMissingSlash)?;

		let (mut record_collection, sha): (BranchCoverageRecordCollection, Option<String>) = {
			let (parts, body) = octocrab::instance()
				.repos(coverage_owner, coverage_repo)
				.raw_file(Reference::Branch(String::from(RECORDS_BRANCH)), report_path.as_str())
				.await?
				.into_parts();
			if parts.status == StatusCode::NOT_FOUND {
				None
			} else if parts.status == StatusCode::UNAUTHORIZED {
				return Err(MeowCoverageError::MissingAccessToCoverageRepo);
			} else {
				let bytes = hyper::body::to_bytes(body).await?;
				let sha = get_file_sha(
					coverage_owner,
					coverage_repo,
					Reference::Branch(String::from(RECORDS_BRANCH)),
					&report_path,
				)
				.await?;
				Some((serde_json::from_slice(&bytes)?, Some(sha)))
			}
		}
		.unwrap_or_else(|| (BranchCoverageRecordCollection { team, records: Vec::new() }, None));

		let mut files = HashMap::new();

		for file_cov in untested_changes {
			files.insert(
				file_cov.realpath.clone(),
				FileCoverageRecord::new(file_cov.percentage, file_cov.raw_lines),
			);
		}

		for file in tested_files {
			files.insert(file, FileCoverageRecord::new(10000_f64, Vec::new()));
		}

		record_collection.add_new_record(lcov.percentage(), files);

		let content = serde_json::to_vec(&record_collection)?;

		match sha {
			Some(sha) => {
				octocrab::instance()
					.repos(coverage_owner, coverage_repo)
					.update_file(
						report_path,
						format!("Add report for {}/{} ({})", coverage_owner, coverage_repo, branch),
						content,
						sha,
					)
					.branch(RECORDS_BRANCH)
					.author(author())
					.commiter(author())
					.send()
					.await?
			}
			None => {
				octocrab::instance()
					.repos(coverage_owner, coverage_repo)
					.create_file(
						report_path,
						format!("Add report for {}/{} ({})", coverage_owner, coverage_repo, branch),
						content,
					)
					.branch(RECORDS_BRANCH)
					.author(author())
					.commiter(author())
					.send()
					.await?
			}
		};

		octocrab::instance()
			.actions()
			.create_workflow_dispatch(coverage_owner, coverage_repo, "main.yml", "main")
			.inputs(
				serde_json::json!({"repo-name": format!("{}/{}", owner, repo), "branch": branch}),
			)
			.send()
			.await?;
	}

	Ok(())
}
