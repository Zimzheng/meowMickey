use serde::{Deserialize, Serialize};
use std::path::Path;

const FILE_NAME: &str = "hydration-records.json";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HydrationRecord {
    pub id: String,
    pub amount_ml: u32,
    pub recorded_at_ms: u64,
    pub local_date: String,
    pub local_time: String,
    pub source: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HydrationData {
    pub schema_version: u8,
    pub records: Vec<HydrationRecord>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HydrationSummary {
    pub local_date: String,
    pub total_ml: u32,
    pub record_count: usize,
    pub records: Vec<HydrationRecord>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HydrationWeek {
    pub days: Vec<HydrationSummary>,
    pub total_ml: u32,
    pub record_count: usize,
    pub recorded_days: usize,
}

// Calendar arithmetic avoids 24-hour timestamp subtraction across DST changes.
pub fn week_summary(data: &HydrationData, end_date: &str) -> Result<HydrationWeek, String> {
    let parts: Vec<_> = end_date.split('-').collect();
    if parts.len() != 3 || end_date.len() != 10 {
        return Err("日期格式无效".into());
    }
    let year: i32 = parts[0].parse().map_err(|_| "日期格式无效")?;
    let month: u8 = parts[1].parse().map_err(|_| "日期格式无效")?;
    let day: u8 = parts[2].parse().map_err(|_| "日期格式无效")?;
    let month = time::Month::try_from(month).map_err(|_| "日期格式无效")?;
    let end = time::Date::from_calendar_date(year, month, day).map_err(|_| "日期格式无效")?;
    let mut days = Vec::with_capacity(7);
    for offset in (0..7).rev() {
        let date = end.checked_sub(time::Duration::days(offset)).ok_or("日期超出范围")?;
        let key = format!("{:04}-{:02}-{:02}", date.year(), date.month() as u8, date.day());
        days.push(summary(data, &key));
    }
    Ok(HydrationWeek {
        total_ml: days.iter().map(|day| day.total_ml).sum(),
        record_count: days.iter().map(|day| day.record_count).sum(),
        recorded_days: days.iter().filter(|day| day.record_count > 0).count(),
        days,
    })
}

fn path(data_dir: &Path) -> std::path::PathBuf { data_dir.join(FILE_NAME) }

pub fn load(data_dir: &Path) -> HydrationData {
    load_checked(data_dir).unwrap_or_default()
}

pub fn load_checked(data_dir: &Path) -> Result<HydrationData, String> {
    match std::fs::read_to_string(path(data_dir)) {
        Ok(text) => serde_json::from_str(&text).map_err(|_| "喝水记录文件暂时无法解析，原记录已保留".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(HydrationData { schema_version: 1, records: vec![] }),
        Err(_) => Err("喝水记录暂时无法读取，请稍后重试".into()),
    }
}

pub fn append(data_dir: &Path, record: HydrationRecord) -> Result<HydrationSummary, String> {
    if !(10..=3000).contains(&record.amount_ml) { return Err("饮水量需要在 10–3000 ml 之间".into()); }
    if record.local_date.len() != 10 || record.local_time.len() < 5 { return Err("日期或时间格式无效".into()); }
    let date = record.local_date.clone();
    let mut data = load_checked(data_dir)?;
    data.schema_version = 1;
    data.records.push(record);
    if data.records.len() > 10_000 { data.records.drain(..data.records.len() - 10_000); }
    save(data_dir, &data)?;
    Ok(summary(&data, &date))
}

fn save(data_dir: &Path, data: &HydrationData) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
    let target = path(data_dir);
    let tmp = target.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, target).map_err(|e| e.to_string())
}

pub fn undo_latest(data_dir: &Path, local_date: &str) -> Result<HydrationSummary, String> {
    let mut data = load_checked(data_dir)?;
    let index = data.records.iter().rposition(|record| record.local_date == local_date)
        .ok_or_else(|| "今天没有可以撤销的喝水记录".to_string())?;
    data.records.remove(index);
    save(data_dir, &data)?;
    Ok(summary(&data, local_date))
}

pub fn summary(data: &HydrationData, local_date: &str) -> HydrationSummary {
    let records: Vec<_> = data.records.iter().filter(|r| r.local_date == local_date).cloned().collect();
    HydrationSummary { local_date: local_date.into(), total_ml: records.iter().map(|r| r.amount_ml).sum(), record_count: records.len(), records }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_and_summarizes_records() {
        let dir = tempfile::tempdir().unwrap();
        let record = HydrationRecord { id: "1".into(), amount_ml: 250, recorded_at_ms: 1, local_date: "2026-09-22".into(), local_time: "10:30".into(), source: "reminder".into() };
        let result = append(dir.path(), record).unwrap();
        assert_eq!(result.total_ml, 250);
        assert_eq!(load(dir.path()).records.len(), 1);
        let result = undo_latest(dir.path(), "2026-09-22").unwrap();
        assert_eq!(result.total_ml, 0);
        assert!(load(dir.path()).records.is_empty());
    }

    fn entry(date: &str, amount_ml: u32) -> HydrationRecord {
        HydrationRecord { id: date.into(), amount_ml, recorded_at_ms: 1, local_date: date.into(), local_time: "10:30".into(), source: "manual".into() }
    }

    #[test]
    fn week_includes_seven_local_dates_and_preserves_missing_days() {
        let data = HydrationData { schema_version: 1, records: vec![
            entry("2026-09-30", 200), entry("2026-10-01", 300),
            entry("2026-10-01", 500), entry("2026-10-06", 250),
            entry("2026-09-29", 900), entry("2026-10-07", 700),
        ] };
        let week = week_summary(&data, "2026-10-06").unwrap();
        assert_eq!(week.days.len(), 7);
        assert_eq!(week.days[0].local_date, "2026-09-30");
        assert_eq!(week.days[1].total_ml, 800);
        assert_eq!(week.days[1].records.len(), 2);
        assert_eq!(week.days[2].record_count, 0);
        assert_eq!(week.total_ml, 1250);
        assert_eq!(week.record_count, 4);
        assert_eq!(week.recorded_days, 3);
        assert_eq!(data.records.len(), 6);
    }

    #[test]
    fn week_handles_leap_days_year_boundaries_and_invalid_dates() {
        let data = HydrationData::default();
        let leap = week_summary(&data, "2024-03-02").unwrap();
        assert_eq!(leap.days[4].local_date, "2024-02-29");
        assert_eq!(leap.recorded_days, 0);
        let year = week_summary(&data, "2026-01-03").unwrap();
        assert_eq!(year.days[0].local_date, "2025-12-28");
        for date in ["2026-02-29", "2026-13-01", "2026-10-32", "2026-1-03", "oops"] {
            assert!(week_summary(&data, date).is_err(), "{date}");
        }
    }

    #[test]
    fn unreadable_history_is_not_replaced_by_a_new_record() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(path(dir.path()), "damaged-history").unwrap();
        assert!(load_checked(dir.path()).is_err());
        assert!(append(dir.path(), entry("2026-10-09", 300)).is_err());
        assert!(undo_latest(dir.path(), "2026-10-09").is_err());
        assert_eq!(std::fs::read_to_string(path(dir.path())).unwrap(), "damaged-history");
    }
}
