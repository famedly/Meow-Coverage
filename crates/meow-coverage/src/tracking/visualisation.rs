//! Module for building the centralised visualisation resources

use std::{borrow::Cow, path::Path};

use time::OffsetDateTime;

use super::{BranchCoverageRecordCollection, PercentWrapper, Team};
use crate::MeowCoverageError;

/// Try and collect records
fn try_collect_records(records: &Path) -> Result<[Vec<ReadmeCoverageEntry>; 6], MeowCoverageError> {
	let mut teams: [Vec<ReadmeCoverageEntry>; 6] =
		[Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];

	let records_dir = std::fs::read_dir(records)?;

	for owner in records_dir {
		let owner = owner?;

		if owner.file_type()?.is_symlink() || !owner.file_type()?.is_dir() {
			continue;
		}

		let owner_dir = std::fs::read_dir(owner.path())?;

		for repo in owner_dir {
			let repo = repo?;

			if repo.file_type()?.is_symlink() || !repo.file_type()?.is_dir() {
				continue;
			}

			let repo_dir = std::fs::read_dir(repo.path())?;

			for branch in repo_dir {
				let branch = branch?;

				if branch.file_type()?.is_symlink()
					|| branch.file_type()?.is_dir()
					|| !branch
						.file_name()
						.to_str()
						.map(|name| name.ends_with(".meowcov.json"))
						.unwrap_or_default()
				{
					continue;
				}

				let owner_file_name = owner.file_name();
				let repo_file_name = repo.file_name();
				let branch_file_name = branch.file_name();

				#[allow(clippy::print_stderr)]
				let Some(owner_name) = owner_file_name.to_str() else {
					eprintln!("Unable to turn {:?} into String", owner_file_name);
					continue;
				};

				#[allow(clippy::print_stderr)]
				let Some(repo_name) = repo_file_name.to_str() else {
					eprintln!("Unable to turn {:?} into String", repo_file_name);
					continue;
				};

				#[allow(clippy::print_stderr)]
				let Some(branch_name) =
					branch_file_name.to_str().map(|value| value.trim_end_matches(".meowcov.json"))
				else {
					eprintln!("Unable to turn {:?} into String", branch_file_name);
					continue;
				};

				let record_collection: BranchCoverageRecordCollection =
					serde_json::from_reader(std::fs::File::open(branch.path())?)?;

				let idx = record_collection.team as usize;
				if let Some(entry) = ReadmeCoverageEntry::from_collection(
					owner_name,
					repo_name,
					branch_name,
					record_collection,
				) {
					teams[idx].push(entry);
				}
			}
		}
	}

	Ok(teams)
}

/// Data needed for each table entry in the README
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadmeCoverageEntry {
	/// Repo Owner
	pub owner: String,
	/// Repo Name
	pub repo: String,
	/// Repo Branch
	pub branch: String,
	/// Coverage
	pub coverage: i16,
	/// Last delta
	pub last_delta: i16,
	/// 7 day delta
	pub delta_7_days: i16,
	/// 30 day delta
	pub delta_30_days: i16,
	/// 90 day delta
	pub delta_90_days: i16,
	/// Latest update date
	pub last_update: OffsetDateTime,
}

impl ReadmeCoverageEntry {
	/// Build [Self] from an `owner`, `repo`, `branch`, and
	/// [BranchCoverageRecordCollection]
	pub fn from_collection(
		owner: &str,
		repo: &str,
		branch: &str,
		record: BranchCoverageRecordCollection,
	) -> Option<Self> {
		Some(Self {
			owner: String::from(owner),
			repo: String::from(repo),
			branch: String::from(branch),
			coverage: record.latest()?.percentage,
			last_delta: record.last_delta()?,
			delta_7_days: record.delta_last_7_days()?,
			delta_30_days: record.delta_last_30_days()?,
			delta_90_days: record.delta_last_90_days()?,
			last_update: OffsetDateTime::from_unix_timestamp(record.latest_timestamp()?).ok()?,
		})
	}
}

/// Builds the table for a team in the README
fn build_team_readme(
	coverage_repo_owner: &str,
	coverage_repo: &str,
	team: Team,
	branches: &[ReadmeCoverageEntry],
) -> String {
	let count = branches.len();

	let table_entries = branches.iter().map(|entry| {
        format!("| [{owner}/{repo} ({branch})](https://github.com/{owner}/{repo}/tree/{branch}) | {cov}% | [Report](https://github.com/{cov_owner}/{cov_repo}/blob/main/reports/{owner}/{repo}/{branch}.md) | {last_delta}%         | {delta7}%         | {delta30}%          | {delta90}%          | {time}   |\n",
            owner = entry.owner,
            repo = entry.repo,
            branch = entry.branch,
            cov = PercentWrapper(entry.coverage),
            last_delta = PercentWrapper(entry.last_delta),
            delta7 = PercentWrapper(entry.delta_7_days),
            delta30 = PercentWrapper(entry.delta_30_days),
            delta90 = PercentWrapper(entry.delta_90_days),
            time = entry.last_update,
            cov_owner = coverage_repo_owner,
            cov_repo = coverage_repo
        )
    }).fold(String::new(), |acc, val| format!("{}{}\n", acc, val));

	format!("\
## {}

Tracking coverage of {} branches of repositories in this group

| Repository (Branch)                | Coverage  | Report         | Delta (Last) | Delta (7 Days) | Delta (30 Days) | Delta (90 Days) | Last Updated |
|------------------------------------|-----------|----------------|--------------|----------------|-----------------|-----------------|--------------|
{}\n",
    team,
    count,
    table_entries
    )
}

/// Builds a new `README.md` into a string
pub fn build_readme(
	records: &Path,
	coverage_repo_owner: &str,
	coverage_repo: &str,
) -> Result<String, MeowCoverageError> {
	let team_records = try_collect_records(records)?;

	let total_count = team_records[Team::InstantMessaging as usize].len()
		+ team_records[Team::Workflow as usize].len()
		+ team_records[Team::Infrastructure as usize].len()
		+ team_records[Team::Product as usize].len()
		+ team_records[Team::Security as usize].len()
		+ team_records[Team::Other as usize].len();

	Ok(format!(
		"\
# Coverage Reports

For a description of this repository please [read here](./Description.md).

Tracking coverage of {} branches of repositories

## Teams

- [Instant Messaging](#instant-messaging)
- [Workflow](#workflow)
- [Infrastructure](#infrastructure)
- [Product](#product)
- [Security](#security)
- [Other](#other)

{im}

{workflow}

{infra}

{product}

{security}

{other}
    ",
		total_count,
		im = build_team_readme(
			coverage_repo_owner,
			coverage_repo,
			Team::InstantMessaging,
			&team_records[Team::InstantMessaging as usize]
		),
		workflow = build_team_readme(
			coverage_repo_owner,
			coverage_repo,
			Team::Workflow,
			&team_records[Team::Workflow as usize]
		),
		infra = build_team_readme(
			coverage_repo_owner,
			coverage_repo,
			Team::Infrastructure,
			&team_records[Team::Infrastructure as usize]
		),
		product = build_team_readme(
			coverage_repo_owner,
			coverage_repo,
			Team::Product,
			&team_records[Team::Product as usize]
		),
		security = build_team_readme(
			coverage_repo_owner,
			coverage_repo,
			Team::Security,
			&team_records[Team::Security as usize]
		),
		other = build_team_readme(
			coverage_repo_owner,
			coverage_repo,
			Team::Other,
			&team_records[Team::Other as usize]
		)
	))
}

/// Build a list of lines
fn build_lines(
	repo_owner: &str,
	repo: &str,
	branch: &str,
	file_path: &str,
	lines: &[u32],
) -> String {
	itertools::intersperse(lines.iter().copied().fold(Vec::<(u32, u32)>::new(), |mut acc, val| {
        match acc.last_mut() {
            Some(last) => {
                if last.1 == val - 1 {
                    last.1 = val;
                } else {
                    acc.push((val, val));
                }
            },
            None => acc.push((val, val))
        }

        acc
    }).into_iter().map(|(start_line, end_line)| {
        match start_line == end_line {
            true => Cow::Owned(format!("[{line}](https://github.com/{repo_owner}/{repo}/blob/{branch}/{file_path}#L{line})", repo_owner = repo_owner, repo = repo, branch = branch, file_path = file_path, line = start_line)),
            false => Cow::Owned(format!("[{start_line}-{end_line}](https://github.com/{repo_owner}/{repo}/blob/{branch}/{file_path}#L{start_line}-L{end_line})", repo_owner = repo_owner, repo = repo, branch = branch, file_path = file_path, start_line = start_line, end_line = end_line)),
        }
    }), Cow::Borrowed(", "))
    .collect()
}

/// Builds individual coverage report markdown files to a string
pub fn build_coverage_report(
	target_repo_owner: &str,
	target_repo: &str,
	branch: &str,
	record_collection: &BranchCoverageRecordCollection,
) -> Option<String> {
	let latest = record_collection.latest()?;
	let time = OffsetDateTime::from_unix_timestamp(latest.timestamp).ok()?;

	let file_cov = latest.files.iter().map(|map| {
            map
                .iter()
                .fold(String::new(), |val, (file_name, value)| val + &format!("| [{file_name}](https://github.com/{repo_owner}/{repo}/blob/{branch}/{file_name}) | {cov}% | {lines} |", file_name = file_name, repo_owner = target_repo_owner, repo = target_repo, branch = branch, cov = PercentWrapper(value.percentage), lines = build_lines(target_repo_owner, target_repo, branch, file_name, &value.untested_lines)))
    }).fold(String::from("| File Name | Coverage  | Untested Lines  |\n|-----------|-----------|-----------------|\n"), |l, r| l + r.as_ref());

	Some(format!(
		"\
# [{repo_owner}/{repo_name}](https://github.com/{repo_owner}/{repo_name}/)

### Branch: `{branch_name}`
### Responsible Team: {team}

#### Last Updated: {timestamp}
#### Coverage: {coverage}%
#### Last Delta: {last_delta}%
#### 7 Day Delta: {delta7}%
#### 30 Day Delta: {delta30}%
#### 90 Day Delta: {delta90}%

{file_cov}\n",
		repo_owner = target_repo_owner,
		repo_name = target_repo,
		branch_name = branch,
		team = record_collection.team,
		coverage = PercentWrapper(latest.percentage),
		timestamp = time,
		last_delta = record_collection.last_delta()?,
		delta7 = record_collection.delta_last_7_days()?,
		delta30 = record_collection.delta_last_30_days()?,
		delta90 = record_collection.delta_last_90_days()?
	))
}
