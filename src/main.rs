use calamine::{Reader, open_workbook_auto};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook =
        open_workbook_auto("/Users/chenxizhou/CS/2026s-OS/AsyncOS/spider/高校名称和官方网站.xlsx")?;
    let range = workbook.worksheet_range_at(0).unwrap()?;
    for row in range.rows().skip(1) {
        let school = row.get(0).map(|c| c.to_string()).unwrap_or_default();
        let url = row.get(1).map(|c| c.to_string()).unwrap_or_default();
    }
    Ok(())
}
