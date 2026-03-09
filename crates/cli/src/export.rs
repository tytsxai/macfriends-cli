use crate::model::{ExportReport, ScanReport};
use anyhow::{Context, Result};
use csv::Writer;
use std::path::Path;

pub fn write_json(report: &ScanReport, output: &Path) -> Result<ExportReport> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, serde_json::to_vec_pretty(report)?)?;
    Ok(ExportReport {
        format: "json".to_string(),
        output: output.display().to_string(),
        records: report.records.len(),
    })
}

pub fn write_csv(report: &ScanReport, output: &Path) -> Result<ExportReport> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = Writer::from_path(output).context("无法创建 CSV 输出")?;
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
            record.wxid.as_str(),
            record.nickname.as_str(),
            record.remark.as_deref().unwrap_or(""),
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
    Ok(ExportReport {
        format: "csv".to_string(),
        output: output.display().to_string(),
        records: report.records.len(),
    })
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
}
