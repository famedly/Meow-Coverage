//! A code coverage visualiser integrated into GitHub

use std::{borrow::Cow, collections::HashMap};

use ::lcov::report::ParseError;
use clap::Parser;
use helpers::{create_review_comment, line_changed_in_hunk, lines_in_same_hunk, path_split};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::lcov::LcovWrapper;

mod helpers;
mod html;
mod lcov;

/// Meow-Coverage CLI Arguments
#[derive(Debug, clap::Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
	/// Prefix for locating source files in Lcov paths (for example 'src/')
	#[clap(long)]
	source_prefix: String,

	/// Repository name in format `OWNER/REPO`
	#[clap(long)]
	repo_name: String,

	/// Commit ID
	#[clap(long)]
	commit_id: String,

	/// GitHub API Token
	#[clap(long)]
	github_token: String,

	/// New Lcov file path
	#[clap(long)]
	new_lcov_file: String,

	/// Choose if Push or PullRequest based
	#[clap(subcommand)]
	command: Commands,
}

/// Subcommand wrapper
#[derive(Debug, clap::Subcommand)]
enum Commands {
	/// Run for a commit
	Push,
	/// Run for a PR
	PullRequest {
		/// Pull request identifier
		#[clap(long)]
		pr_number: u64,

		/// Old Lcov file path
		#[clap(long)]
		old_lcov_file: Option<String>,
	},
}

/// Error collection
#[derive(Debug, Error)]
pub enum MeowCoverageError {
	/// LCOV parsing error ([ParseError])
	#[error("Lcov Parsing Failed: {0}")]
	LcovParse(#[from] ParseError),
	/// GitHub API Error ([octocrab::Error])
	#[error("GitHub API Error: {0}")]
	GitHub(#[from] octocrab::Error),
	/// Repository name not in format `OWNER/REPO`
	#[error("Repo Name must be in format OWNER/REPO")]
	RepoNameMissingSlash,
	/// Patch parsing error [patch::ParseError]
	#[error("Patch Parse Error: {0}")]
	Patch(String),
}

impl From<patch::ParseError<'_>> for MeowCoverageError {
	fn from(value: patch::ParseError<'_>) -> Self {
		Self::Patch(format!("{}", value))
	}
}

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

/// File coverage wrapper for commits
#[derive(Debug)]
pub struct PushFileCoverageWrapper {
	/// File Git SHA
	pub sha: String,
	/// Collection of unclumped lines
	pub raw_lines: Vec<u32>,
	/// File Path
	pub realpath: String,
}

/// Generates a report for a Pull Request
#[allow(clippy::too_many_lines)]
async fn generate_pr_coverage_report(
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
				let patch = match patch::Patch::from_single(&patch_str) {
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

/// Generates a report for a commit
async fn generate_push_coverage_report(
	lcov_path: &str,
	repo_name: &str,
	source_prefix: &str,
	commit_sha: &str,
) -> Result<(), MeowCoverageError> {
	let lcov = LcovWrapper::new(lcov_path)?;

	let (owner, repo) = repo_name.split_once('/').ok_or(MeowCoverageError::RepoNameMissingSlash)?;

	let untested_changes = lcov
		.group_data()
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
					false => Cow::Owned(html::build_push_summary(
						owner,
						repo,
						commit_sha,
						&untested_changes
					)),
				}
			),
		)
		.send()
		.await?;

	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), MeowCoverageError> {
	let args = CliArgs::parse();

	octocrab::initialise(octocrab::Octocrab::builder().personal_token(args.github_token).build()?);

	match args.command {
		Commands::PullRequest { pr_number, old_lcov_file } => {
			generate_pr_coverage_report(
				args.repo_name.as_str(),
				args.source_prefix.as_str(),
				args.commit_id.as_str(),
				pr_number,
				args.new_lcov_file.as_str(),
				old_lcov_file.as_deref(),
			)
			.await?;
		}
		Commands::Push => {
			generate_push_coverage_report(
				args.new_lcov_file.as_str(),
				args.repo_name.as_str(),
				args.source_prefix.as_str(),
				args.commit_id.as_str(),
			)
			.await?;
		}
	}

	Ok(())
}
