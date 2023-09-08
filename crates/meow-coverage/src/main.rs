//! A code coverage visualiser integrated into GitHub

use std::path::PathBuf;

use clap::Parser;
use tracking::Team;

mod api;
mod coverage;
mod tracking;

/// Meow-Coverage CLI Main Command
#[derive(Debug, clap::Subcommand)]
enum CliMainCommand {
	/// Centralised coverage tracking repo operations
	Tracking {
		/// Coverage repository in format `OWNER/REPO`
		#[clap(long)]
		coverage_repo_name: String,
		/// Tracking subcommand
		#[clap(subcommand)]
		command: CliTrackingCommand,
	},
	/// Analyse coverage for a single run
	CoverageRun {
		/// Prefix for locating source files in Lcov paths (for example 'src/')
		#[clap(long)]
		source_prefix: String,

		/// Commit ID
		#[clap(long)]
		commit_id: String,

		/// New Lcov file path
		#[clap(long)]
		new_lcov_file: String,

		/// Choose if Push or PullRequest based
		#[clap(subcommand)]
		command: CliCoverageCommand,
	},
}

/// Meow-Coverage CLI Arguments
#[derive(Debug, clap::Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
	/// GitHub API Token
	#[clap(long)]
	github_token: String,

	/// Repository name in format `OWNER/REPO`
	#[clap(long)]
	repo_name: String,

	/// Choose if analysing coverage for a single run, or managing the
	/// centralised coverage tracking repo
	#[clap(subcommand)]
	command: CliMainCommand,
}

/// Subcommand wrapper for managing the centralised coverage tracking repo
#[derive(Debug, clap::Subcommand)]
enum CliTrackingCommand {
	/// Rebuild the `main` branch, this should only be called by the GitHub
	/// action for the repo
	Rebuild {
		/// Path to where the `records` branch of the tracking repository is
		/// cloned
		#[clap(long = "records")]
		tracking_repo_records: PathBuf,

		/// Repository branch to generate individualised report on
		#[clap(long)]
		branch: String,
	},
	/// Remove a branch of a repository from the tracking records
	RemoveBranch {
		/// Repository branch to remove
		#[clap(long)]
		branch: String,
	},
}

/// Subcommand wrapper for coverage run operations
#[derive(Debug, clap::Subcommand)]
enum CliCoverageCommand {
	/// Run for a commit
	Push,
	/// Run for a commit and collect the report afterwards
	PushWithReport {
		/// Branch for the commit
		#[clap(long)]
		branch: String,
		/// Repository for submitting the coverage report record to
		#[clap(long)]
		coverage_repo: String,
		/// Repository for submitting the coverage report record to
		#[clap(long)]
		coverage_team: Team,
	},
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
#[derive(Debug, thiserror::Error)]
pub enum MeowCoverageError {
	/// LCOV parsing error ([ParseError])
	#[error("Lcov Parsing Failed: {0}")]
	LcovParse(#[from] meow_coverage_shared::lcov::report::ParseError),
	/// GitHub API Error ([octocrab::Error])
	#[error("GitHub API Error: {0}")]
	GitHub(#[from] octocrab::Error),
	/// Repository name not in format `OWNER/REPO`
	#[error("Repo Name must be in format OWNER/REPO")]
	RepoNameMissingSlash,
	/// Patch parsing error [patch::ParseError]
	#[error("Patch Parse Error: {0}")]
	Patch(String),
	/// Hyper error
	#[error("Hyper Error: {0}")]
	Hyper(#[from] hyper::Error),
	/// serde_json error
	#[error("Serde Error: {0}")]
	SerdeJson(#[from] serde_json::Error),
	/// GitHub token does not have permission to access the contents of the
	/// coverage repo
	#[error("Token does not have permission to access coverage repo")]
	MissingAccessToCoverageRepo,
	/// [std::io::Error] vairant
	#[error(transparent)]
	Io(#[from] std::io::Error),
	/// Attempted to build a report on a branch that is missing valid reports
	#[error("Attempted to build a report on a branch that is missing valid reports")]
	ReportMissingInfo,
}

impl From<meow_coverage_shared::patch::ParseError<'_>> for MeowCoverageError {
	fn from(value: meow_coverage_shared::patch::ParseError<'_>) -> Self {
		Self::Patch(format!("{}", value))
	}
}

#[tokio::main]
async fn main() -> Result<(), MeowCoverageError> {
	let args = CliArgs::parse();

	octocrab::initialise(octocrab::Octocrab::builder().personal_token(args.github_token).build()?);

	match args.command {
		CliMainCommand::Tracking { coverage_repo_name, command } => match command {
			CliTrackingCommand::Rebuild { tracking_repo_records, branch } => {
				tracking::rebuild(
					&tracking_repo_records,
					coverage_repo_name.as_str(),
					args.repo_name.as_str(),
					branch.as_str(),
				)
				.await
			}
			CliTrackingCommand::RemoveBranch { branch } => {
				tracking::remove_branch_from_tracking(
					coverage_repo_name.as_str(),
					args.repo_name.as_str(),
					branch.as_str(),
				)
				.await
			}
		},
		CliMainCommand::CoverageRun { source_prefix, commit_id, new_lcov_file, command } => {
			match command {
				CliCoverageCommand::PullRequest { pr_number, old_lcov_file } => {
					coverage::generate_pr_coverage_report(
						args.repo_name.as_str(),
						source_prefix.as_str(),
						commit_id.as_str(),
						pr_number,
						new_lcov_file.as_str(),
						old_lcov_file.as_deref(),
					)
					.await
				}
				CliCoverageCommand::Push => {
					coverage::generate_push_coverage_report(
						new_lcov_file.as_str(),
						args.repo_name.as_str(),
						source_prefix.as_str(),
						commit_id.as_str(),
						None,
					)
					.await
				}
				CliCoverageCommand::PushWithReport { branch, coverage_repo, coverage_team } => {
					coverage::generate_push_coverage_report(
						new_lcov_file.as_str(),
						args.repo_name.as_str(),
						source_prefix.as_str(),
						commit_id.as_str(),
						Some((branch.as_str(), coverage_repo.as_str(), coverage_team)),
					)
					.await
				}
			}
		}
	}
}
