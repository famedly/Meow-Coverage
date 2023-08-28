//! This module contains functions for managing repositories in the centralised
//! coverage tracking records

use octocrab::params::repos::Reference;

use super::{author, make_report_path, RECORDS_BRANCH};
use crate::{github_api::get_file_sha, MeowCoverageError};

/// Remove a branch of a repository from the centralised coverage tracking
/// records
pub async fn remove_branch_from_tracking(
	coverage_repo_name: &str,
	remove_target_repo_name: &str,
	remove_target_branch: &str,
) -> Result<(), MeowCoverageError> {
	let (coverage_repo_owner, coverage_repo) =
		coverage_repo_name.split_once('/').ok_or(MeowCoverageError::RepoNameMissingSlash)?;
	let (remove_target_repo_owner, remove_target_repo) =
		remove_target_repo_name.split_once('/').ok_or(MeowCoverageError::RepoNameMissingSlash)?;

	let report_path =
		make_report_path(remove_target_repo_owner, remove_target_repo, remove_target_branch);

	let sha = get_file_sha(
		coverage_repo_owner,
		coverage_repo,
		Reference::Branch(String::from(RECORDS_BRANCH)),
		&report_path,
	)
	.await?;

	octocrab::instance()
		.repos(coverage_repo_owner, coverage_repo)
		.delete_file(
			report_path,
			format!(
				"Delete report for {}/{} ({})",
				remove_target_repo_owner, remove_target_repo, remove_target_branch
			),
			sha,
		)
		.branch(RECORDS_BRANCH)
		.author(author())
		.commiter(author())
		.send()
		.await?;

	Ok(())
}
