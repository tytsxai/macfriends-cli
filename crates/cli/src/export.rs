use crate::model::{ExportReport, ScanReport};
use crate::util;
use anyhow::{Context, Result};
use csv::Writer;
use std::path::Path;

pub fn write_json(report: &ScanReport, output: &Path) -> Result<ExportReport> {
    util::write_bytes_atomic(output, &serde_json::to_vec_pretty(report)?)?;
    Ok(ExportReport {
        format: "json".to_string(),
        output: output.display().to_string(),
        records: report.records.len(),
    })
}

pub fn write_csv(report: &ScanReport, output: &Path) -> Result<ExportReport> {
    let mut writer = Writer::from_writer(Vec::new());
    writer.write_record([
        "wxid",
        "nickname",
        "remark",
        "status",
        "status_code",
        "source_version",
        "scanned_at",
    ])?;
    for record in &report.records {
        writer.write_record([
            sanitize_csv_cell(&record.wxid).as_str(),
            sanitize_csv_cell(&record.nickname).as_str(),
            sanitize_csv_cell(record.remark.as_deref().unwrap_or("")).as_str(),
            match record.status {
                crate::model::FriendStatus::Normal => "normal",
                crate::model::FriendStatus::Deleted => "deleted",
                crate::model::FriendStatus::Blocked => "blocked",
                crate::model::FriendStatus::Unknown => "unknown",
            },
            record.status_code.as_str(),
            record.source_version.as_str(),
            &record.scanned_at.to_rfc3339(),
        ])?;
    }
    writer.flush()?;
    let content = writer.into_inner().context("无法刷新 CSV 输出缓冲区")?;
    util::write_bytes_atomic(output, &content)?;
    Ok(ExportReport {
        format: "csv".to_string(),
        output: output.display().to_string(),
        records: report.records.len(),
    })
}

fn sanitize_csv_cell(value: &str) -> String {
    let trimmed = value.trim_start_matches([' ', '\t', '\r']);
    if matches!(trimmed.as_bytes().first(), Some(b'=' | b'+' | b'-' | b'@')) {
        format!("'{value}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FriendRecord, FriendStatus};
    use chrono::Utc;
    use std::collections::BTreeMap;

    #[test]
    fn json_export_works() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("scan.json");
        let report = ScanReport {
            mode: "production".into(),
            run_id: "run-test".into(),
            adapter_name: "wechat_4_1_8_arm64".into(),
            source_version: "3.0.0".into(),
            scanned_at: Utc::now(),
            summary: BTreeMap::from([("normal".to_string(), 1)]),
            records: vec![FriendRecord {
                wxid: "wxid_1".into(),
                nickname: "Alice".into(),
                remark: Some("A".into()),
                status: FriendStatus::Normal,
                status_code: "0xB1".into(),
                source_version: "3.0.0".into(),
                scanned_at: Utc::now(),
            }],
        };
        let export = write_json(&report, &path).unwrap();
        assert_eq!(export.records, 1);
        assert!(path.exists());
    }

    #[test]
    fn csv_export_works() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("scan.csv");
        let report = ScanReport {
            mode: "production".into(),
            run_id: "run-test".into(),
            adapter_name: "wechat_4_1_8_arm64".into(),
            source_version: "3.0.0".into(),
            scanned_at: Utc::now(),
            summary: BTreeMap::from([("normal".to_string(), 1)]),
            records: vec![FriendRecord {
                wxid: "wxid_1".into(),
                nickname: "Alice".into(),
                remark: Some("A".into()),
                status: FriendStatus::Normal,
                status_code: "0xB1".into(),
                source_version: "3.0.0".into(),
                scanned_at: Utc::now(),
            }],
        };
        let export = write_csv(&report, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(export.records, 1);
        assert!(content.contains("wxid_1"));
    }

    #[test]
    fn csv_export_neutralizes_spreadsheet_formulas() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("scan.csv");
        let report = ScanReport {
            mode: "production".into(),
            run_id: "run-test".into(),
            adapter_name: "wechat_4_1_8_arm64".into(),
            source_version: "3.0.0".into(),
            scanned_at: Utc::now(),
            summary: BTreeMap::from([("unknown".to_string(), 1)]),
            records: vec![FriendRecord {
                wxid: "wxid_1".into(),
                nickname: "=cmd".into(),
                remark: Some(" @lookup".into()),
                status: FriendStatus::Unknown,
                status_code: "0x00".into(),
                source_version: "3.0.0".into(),
                scanned_at: Utc::now(),
            }],
        };

        write_csv(&report, &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("'=cmd"));
        assert!(content.contains("' @lookup"));
    }
}
