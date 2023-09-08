//! Module contains definitions for coverage operations on pull requests

use std::{borrow::Cow, collections::HashMap};

use meow_coverage_shared::{line_changed_in_hunk, lines_in_same_hunk, path_split, LcovWrapper};
use sha2::{Digest, Sha256};

use super::html;
use crate::{api::create_review_comment, MeowCoverageError};

/// File coverage wrapper for PRs
#[derive(Debug)]
pub struct PullFileCoverageWrapper {
	/// File Git SHA
	pub sha: String,
	/// Lines collected by range by hunk, the hunking is a limitation of the
	/// GitHub API sadly
	pub hunked_lines: Vec<(u32, u32)>,
	/// Collection of unclumped lines
	pub raw_lines: Vec<u32>,
	/// File path
	pub realpath: String,
}

/// Generates a report for a Pull Request
#[allow(clippy::too_many_lines)]
pub async fn generate_pr_coverage_report(
	repo_name: &str,
	source_prefix: &str,
	commit_id: &str,
	pr_number: u64,
	new_lcov_file: &str,
	old_lcov_file: Option<&str>,
) -> Result<(), MeowCoverageError> {
	let new_lcov = LcovWrapper::new(new_lcov_file)?;

	let percentage_difference = match old_lcov_file {
		Some(old_lcov_file) => {
			Some(LcovWrapper::new(old_lcov_file)?.percentage_difference(&new_lcov))
		}
		None => None,
	};

	let (owner, repo) = repo_name.split_once('/').ok_or(MeowCoverageError::RepoNameMissingSlash)?;

	let untested_changes = {
		let file_diff_meta = octocrab::instance()
			.pulls(owner, repo)
			.list_files(pr_number)
			.await?
			.into_iter()
			.filter_map(|file_diff| {
				file_diff.patch.map(|patch| {
					let patch = format!(
						"--- a/{}\n+++ b/{}\n{}",
						file_diff.previous_filename.as_deref().unwrap_or(&file_diff.filename),
						file_diff.filename,
						patch
					);

					(file_diff.filename, patch)
				})
			})
			.collect::<HashMap<_, _>>();

		let grouped_data = new_lcov.group_data();

		grouped_data
			.into_iter()
			.filter_map(|coverage| {
				let path = path_split(coverage.filename.as_str(), source_prefix);

				let patch_str =
					file_diff_meta.get(&path).map(|patch| match patch.ends_with('\n') {
						true => patch.clone(),
						false => format!("{}\n", patch),
					})?;

				#[allow(clippy::print_stderr)]
				let patch = match meow_coverage_shared::patch::Patch::from_single(&patch_str) {
					Ok(patch) => patch,
					Err(why) => {
						eprintln!("Error parsing patch, continuing with next (why: {})", why);
						return None;
					}
				};

				let raw_lines: Vec<_> = coverage
					.lines
					.into_iter()
					.filter(|line| {
						patch.hunks.iter().any(|hunk| line_changed_in_hunk(hunk, u64::from(*line)))
					})
					.collect();

				if raw_lines.is_empty() {
					return None;
				}

				let hunked_lines: Vec<(u32, u32)> =
					raw_lines.iter().copied().fold(Vec::new(), |mut hunked_lines, line| {
						if let Some(last) = hunked_lines.last_mut() {
							if lines_in_same_hunk(&patch.hunks, u64::from(last.1), u64::from(line))
							{
								last.1 = line;
								return hunked_lines;
							}
						}

						hunked_lines.push((line, line));
						hunked_lines
					});

				Some(PullFileCoverageWrapper {
					hunked_lines,
					raw_lines,
					sha: {
						let mut hasher = Sha256::new();
						hasher.update(path.as_str());
						hex::encode(hasher.finalize())
					},
					realpath: path,
				})
			})
			.collect::<Vec<_>>()
	};

	octocrab::instance()
		.issues(owner, repo)
		.create_comment(
			pr_number,
			format!(
				"<h3>Meow! Coverage</h3>Total: {:.2}%\n\n{}\n\n{}",
				new_lcov.percentage(),
				match percentage_difference {
					Some(delta) => Cow::Owned(format!("Delta: {:.2}%\n\n", delta)),
					None => Cow::Borrowed(""),
				},
				match untested_changes.is_empty() {
					true => Cow::Borrowed("🐾 All changes are tested! 🐾"),
					false => Cow::Owned(html::build_pull_summary(
						owner,
						repo,
						pr_number,
						&untested_changes
					)),
				}
			),
		)
		.await?;

	for change in untested_changes {
		for (first_line, final_line) in change.hunked_lines {
			create_review_comment(
				owner,
				repo,
				pr_number,
				commit_id,
				change.realpath.as_str(),
				first_line,
				final_line,
			)
			.await?;
		}
	}

	Ok(())
}
