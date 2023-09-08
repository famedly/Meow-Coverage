//! This module contains models for record storage

use std::{collections::HashMap, str::FromStr};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::make_percent;

/// Enum for all the teams a project can be owned by
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum Team {
	/// Projects maintained by Instant Messaging team
	InstantMessaging,
	/// Projects maintained by Workflow team
	Workflow,
	/// Projects maintained by Infrastructure team
	Infrastructure,
	/// Projects maintained by Product team
	Product,
	/// Projects maintained by Security team
	Security,
	/// All other projects
	Other,
}

impl std::fmt::Display for Team {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			Self::InstantMessaging => "Instant Messaging",
			Self::Workflow => "Workflow",
			Self::Infrastructure => "Infrastructure",
			Self::Product => "Product",
			Self::Security => "Security",
			Self::Other => "Other",
		})
	}
}

/// Wrapper for errors returned from [Team::from_str]
#[derive(Debug)]
pub struct TeamFromStrError;

impl std::fmt::Display for TeamFromStrError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Invalid Team (expected one of `InstantMessaging`, `Workflow`, `Infrastructure`, `Product`, `Security`, `Other`)")
	}
}

impl std::error::Error for TeamFromStrError {}

impl FromStr for Team {
	type Err = TeamFromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"InstantMessaging" => Ok(Self::InstantMessaging),
			"Workflow" => Ok(Self::Workflow),
			"Infrastructure" => Ok(Self::Infrastructure),
			"Product" => Ok(Self::Product),
			"Security" => Ok(Self::Security),
			"Other" => Ok(Self::Other),
			_ => Err(TeamFromStrError),
		}
	}
}

/// A coverage record for a file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FileCoverageRecord {
	/// File coverage percentage (lines_hit/lines_found)
	pub percentage: i16,
	/// List of untested lines
	pub untested_lines: Vec<u32>,
}

impl FileCoverageRecord {
	/// Create a new [FileCoverageRecord]
	#[must_use]
	pub fn new(percentage: f64, untested_lines: Vec<u32>) -> Self {
		Self { percentage: make_percent(percentage), untested_lines }
	}
}

/// A coverage record for a branch
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BranchCoverageRecord {
	/// Timestamp the record was produced at
	pub timestamp: i64,
	/// Coverage percentage multiplied by 100 stored as an i16
	pub percentage: i16,
	/// List of file coverage records, only present on newest entry
	#[serde(skip_serializing_if = "Option::is_none")]
	pub files: Option<HashMap<String, FileCoverageRecord>>,
}

/// A collection of the records for a branch of a file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BranchCoverageRecordCollection {
	/// Team who is currently responsible for this project's branch
	pub team: Team,
	/// Branch records
	#[serde(skip_serializing_if = "Vec::is_empty", default)]
	pub records: Vec<BranchCoverageRecord>,
}

impl BranchCoverageRecordCollection {
	/// Add a new record, purge old records
	pub fn add_new_record(&mut self, percentage: f64, files: HashMap<String, FileCoverageRecord>) {
		let time: time::OffsetDateTime = time::OffsetDateTime::now_utc();
		let timestamp = time.unix_timestamp();

		self.records.push(BranchCoverageRecord {
			timestamp,
			percentage: make_percent(percentage),
			files: Some(files),
		});

		self.remove_old_records(time);

		#[allow(clippy::expect_used)]
		let highest_ts = self.latest_timestamp().expect("We pushed a record, there is always a timestamp");

		// Remove the file info for old records
		for record in &mut self.records {
			if record.timestamp != highest_ts {
				record.files = None;
			}
		}
	}

	/// Removes records over 90 days old
	pub fn remove_old_records(&mut self, current_time: time::OffsetDateTime) {
		let time_limit = (current_time - time::Duration::days(90)).unix_timestamp();
		self.records.retain(|item| item.timestamp >= time_limit);
	}

	/// Fetch the timestamp of the latest change
	#[must_use]
	pub fn latest(&self) -> Option<&BranchCoverageRecord> {
		self.records.iter().fold(None, |oldest, record| {
			Some(oldest.map_or(record, |oldest| match oldest.timestamp < record.timestamp {
				true => record,
				false => oldest,
			}))
		})
	}

	/// Fetch the timestamp of the latest change
	#[must_use]
	pub fn latest_timestamp(&self) -> Option<i64> {
		self.records.iter().map(|entry| entry.timestamp).sorted_by(|l, r| Ord::cmp(r, l)).next()
	}

	/// Returns the delta of the previous two changes
	#[must_use]
	pub fn last_delta(&self) -> Option<i16> {
		let (Some(newest), second_newest) = ({
			let mut iter =
				self.records.iter().sorted_by(|l, r| Ord::cmp(&r.timestamp, &l.timestamp));
			(iter.next(), iter.next())
		}) else {
			return None;
		};

		Some(match second_newest {
			Some(second_newest) => newest.percentage - second_newest.percentage,
			None => newest.percentage,
		})
	}

	/// Returns the delta of changes between the start and end timestamps
	#[must_use]
	pub fn delta(&self, period_start_ts: i64, period_end_ts: i64) -> Option<i16> {
		let (Some(oldest), newest) = ({
			let mut iter = self
				.records
				.iter()
				.filter(|item| item.timestamp >= period_start_ts && item.timestamp <= period_end_ts)
				.sorted_by(|l, r| Ord::cmp(&l.timestamp, &r.timestamp));
			(iter.next(), iter.last())
		}) else {
			return None;
		};

		Some(match newest {
			Some(newest) => newest.percentage - oldest.percentage,
			None => oldest.percentage,
		})
	}

	/// Returns a delta with a given duration since the last edit
	#[must_use]
	pub fn delta_duration(&self, duration: time::Duration) -> Option<i16> {
		let period_end_ts = self.latest_timestamp()?;
		let period_start_ts = period_end_ts - duration.as_seconds_f64() as i64;

		self.delta(period_start_ts, period_end_ts)
	}

	/// Returns the delta in the past 7 days since the last edit
	#[must_use]
	pub fn delta_last_7_days(&self) -> Option<i16> {
		self.delta_duration(time::Duration::days(7))
	}

	/// Returns the delta in the past 30 days since the last edit
	#[must_use]
	pub fn delta_last_30_days(&self) -> Option<i16> {
		self.delta_duration(time::Duration::days(30))
	}

	/// Returns the delta in the past 90 days since the last edit
	#[must_use]
	pub fn delta_last_90_days(&self) -> Option<i16> {
		self.delta_duration(time::Duration::days(90))
	}
}
