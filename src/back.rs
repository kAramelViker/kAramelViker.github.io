use chrono::{Datelike, Timelike};
use chrono::{Duration, NaiveDate, NaiveDateTime, Weekday};
use regex::Regex;
use std::str::FromStr;
use uuid::Uuid;

use crate::front::{SearchEntryEditor, SortBy, SpendingCategory};

fn gen_uuid() -> Uuid {
	Uuid::new_v4()
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PurchaseEntry {
	#[serde(default = "gen_uuid", skip)]
	pub(crate) id: Uuid,

	pub(crate) date: NaiveDateTime,
	pub(crate) amount: f64,
	pub(crate) merchant: String,
	pub(crate) category: SpendingCategory,
	pub(crate) notes: String,
}

#[derive(Default)]
pub struct AppStorage {
	loaded_data: Vec<PurchaseEntry>,
}

impl AppStorage {
	pub fn add(&mut self, entry: PurchaseEntry) {
		self.loaded_data.push(entry);
	}

	pub fn remove(&mut self, id: Uuid) {
		if let Some(i) = self.loaded_data.iter().position(|item| item.id == id) {
			self.loaded_data.remove(i);
		}
	}

	pub fn get(&self, id: Option<Uuid>) -> Option<&PurchaseEntry> {
		let id = id?;
		self.loaded_data.iter().position(|item| item.id == id)
			.map(|i| &self.loaded_data[i])
	}

	pub fn get_mut(&mut self, id: Option<Uuid>) -> Option<&mut PurchaseEntry> {
		let id = id?;
		self.loaded_data.iter_mut().find(|item| item.id == id)
	}

	pub fn purge(&mut self) {
		self.loaded_data.clear();
	}

	pub fn get_all(&self) -> &[PurchaseEntry] {
		&self.loaded_data
	}

	pub fn get_sorted_filtered(
		&self,
		sort_by: SortBy,
		se: SearchEntryEditor,
	) -> Vec<PurchaseEntry> {
		let mut entries: Vec<PurchaseEntry> = self.get_all().iter().cloned().collect();
		entries.sort_by(|a, b| match sort_by {
			SortBy::DateAsc => a.date.cmp(&b.date),
			SortBy::DateDesc => b.date.cmp(&a.date),

			SortBy::AmountAsc => a
			.amount
			.partial_cmp(&b.amount)
			.unwrap_or(std::cmp::Ordering::Equal),
			SortBy::AmountDesc => b
			.amount
			.partial_cmp(&a.amount)
			.unwrap_or(std::cmp::Ordering::Equal),

			SortBy::MerchantAsc => a.merchant.cmp(&b.merchant),
			SortBy::MerchantDesc => b.merchant.cmp(&a.merchant),

			SortBy::CategoryAsc => a.category.as_str().cmp(b.category.as_str()),
			SortBy::CategoryDesc => b.category.as_str().cmp(a.category.as_str()),
		});

		let se_f_chrono = NaiveDate::from_ymd_opt(
			se.date.year() as i32,
			se.date.month() as u32,
			se.date.day() as u32,
		)
		.expect("invalid start date")
		.and_hms_opt(se.time.hour(), se.time.minute(), se.time.second())
		.unwrap();

		let se_t_chrono = NaiveDate::from_ymd_opt(
			se.date_end.year() as i32,
			se.date_end.month() as u32,
			se.date_end.day() as u32,
		)
		.expect("invalid end date")
		.and_hms_opt(se.time_end.hour(), se.time_end.minute(), se.time_end.second())
		.unwrap();

		let merchant_re = if se.merchant.trim().is_empty() {
			None
		} else {
			Some(Regex::from_str(&se.merchant).unwrap())
		};

		let amount_re = if se.amount_text.trim().is_empty() {
			None
		} else {
			Some(Regex::from_str(&se.amount_text).unwrap())
		};

		// only compile regex if category is not All
		let category_re = match se.category {
			SpendingCategory::All => None,
			_ => Some(Regex::from_str(se.category.as_str()).unwrap()),
		};

		let notes_re = if se.notes.trim().is_empty() {
			None
		} else {
			Some(Regex::from_str(&se.notes).unwrap())
		};

		let mut filtered: Vec<PurchaseEntry> = vec![];

		for entry in entries {
			let merchant_ok = merchant_re.as_ref().map_or(true, |re| re.is_match(&entry.merchant));
			let amount_ok   = amount_re.as_ref().map_or(true, |re| re.is_match(&entry.amount.to_string()));
			let category_ok = category_re.as_ref().map_or(true, |re| re.is_match(entry.category.as_str()));
			let notes_ok    = notes_re.as_ref().map_or(true, |re| re.is_match(&entry.notes));

			let date_ok = !(entry.date < se_f_chrono || entry.date > se_t_chrono);

			let should_add = date_ok && merchant_ok && amount_ok && category_ok && notes_ok;

			if should_add {
				filtered.push(entry);
			}
		}

		filtered
	}

	}

pub fn bounds_today() -> (NaiveDateTime, NaiveDateTime) {
	let now_local = chrono::Local::now().naive_local();
	let d = now_local.date();

	let start = d.and_hms_opt(0, 0, 0).unwrap();
	let end = (d + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap();

	(start, end)
}

pub fn bounds_week_iso_monday() -> (NaiveDateTime, NaiveDateTime) {
	let date: NaiveDate = chrono::Local::now().date_naive();
	let weekday = date.weekday();

	let days_from_monday = match weekday {
		Weekday::Mon => 0,
		Weekday::Tue => 1,
		Weekday::Wed => 2,
		Weekday::Thu => 3,
		Weekday::Fri => 4,
		Weekday::Sat => 5,
		Weekday::Sun => 6,
	};

	let monday_date = date - Duration::days(days_from_monday as i64);
	let start = monday_date.and_hms_opt(0, 0, 0).unwrap();
	let end = (monday_date + Duration::days(7)).and_hms_opt(0, 0, 0).unwrap();

	(start, end)
}

pub fn bounds_month() -> (NaiveDateTime, NaiveDateTime) {
	let now_local = chrono::Local::now().naive_local();
	let d = now_local.date();

	let start_date = NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap();
	let start = start_date.and_hms_opt(0, 0, 0).unwrap();

	let (ny, nm) = if d.month() == 12 {
		(d.year() + 1, 1)
	} else {
		(d.year(), d.month() + 1)
	};

	let end_date = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
	let end = end_date.and_hms_opt(0, 0, 0).unwrap();

	(start, end)
}

pub fn bounds_year() -> (NaiveDateTime, NaiveDateTime) {
	let now_local = chrono::Local::now().naive_local();
	let year = now_local.year();

	let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
	let start = start_date.and_hms_opt(0, 0, 0).unwrap();

	let end_date = NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap();
	let end = end_date.and_hms_opt(0, 0, 0).unwrap();

	(start, end)
}

pub fn bounds_none() -> (NaiveDateTime, NaiveDateTime) {
	(NaiveDateTime::MIN, NaiveDateTime::MAX)
}

pub fn filter_range(
	entries: &[PurchaseEntry],
	start: NaiveDateTime,
	end: NaiveDateTime,
) -> Vec<&PurchaseEntry> {
	entries
		.iter()
		.filter(|e| e.date >= start && e.date < end)
		.collect()
}

pub(crate) fn fingerprint(storage: &AppStorage) -> u64 {
	use std::hash::{Hash, Hasher};
	let mut h = std::collections::hash_map::DefaultHasher::new();

	for e in storage.get_all() {
		e.category.hash(&mut h);
		e.amount.to_bits().hash(&mut h);
		e.date.hash(&mut h);
	}
	h.finish()
}

pub fn new_data() -> PurchaseEntry {
	PurchaseEntry::default()
}

pub fn parse_data(path: Option<String>) -> Vec<PurchaseEntry> {
	let path = path.expect("missing path");

	let rdr = csv::ReaderBuilder::new()
		.has_headers(true)
		.from_path(path);

	let mut rdr = match rdr {
			Err(_error) => {
				return Vec::new();
			},
			Ok(rdr) => rdr,
		};

	let mut entries = Vec::new();
	for result in rdr.deserialize() {
		let record: PurchaseEntry = result.expect("REASON");
		entries.push(record);
	}
	entries
}

pub fn write_data(
	path: Option<String>,
	entries: &[PurchaseEntry],
) -> Result<(), Box<dyn std::error::Error>> {
	let path = path.expect("missing path");

	let mut wtr = csv::WriterBuilder::new()
		.has_headers(true)
		.from_path(path)
		.expect("cannot open csv");

	for entry in entries {
		wtr.serialize(entry)?;
	}

	wtr.flush()?;
	Ok(())
}