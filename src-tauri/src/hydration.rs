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

fn path(data_dir: &Path) -> std::path::PathBuf { data_dir.join(FILE_NAME) }

pub fn load(data_dir: &Path) -> HydrationData {
    std::fs::read_to_string(path(data_dir)).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(HydrationData { schema_version: 1, records: vec![] })
}

pub fn append(data_dir: &Path, record: HydrationRecord) -> Result<HydrationSummary, String> {
    if !(10..=3000).contains(&record.amount_ml) { return Err("饮水量需要在 10–3000 ml 之间".into()); }
    if record.local_date.len() != 10 || record.local_time.len() < 5 { return Err("日期或时间格式无效".into()); }
    let date = record.local_date.clone();
    let mut data = load(data_dir);
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
    let mut data = load(data_dir);
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
}
