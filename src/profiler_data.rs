use crate::BlockStat;
use std::{
    time::Instant,
    rc::Rc,
    cell::RefCell,
};

const HTML_REPORT_HEADER: &str = r#"<html><body>
<title>Profile report</title>

<style>
    body {
        color: #111;
        font-family: Noto Mono;
    }
    tr:nth-child(even) {
        background: #efeeef;
    }
    tr:nth-child(odd) {
        background: #fff;
    }
    td:nth-child(1) {
        font-weight: bold;
        text-align: left;
    }
    td:nth-child(n+2) {
        text-align: right;
    }
</style>

<h1>Functions statistics</h1>
"#;

const HTML_REPORT_FOOTER: &str = "</body></html>";

pub struct ProfilerData {
    pub(crate) main_start_time: Instant,
    pub(crate) main_block: BlockStat,
    pub(crate) blocks_stack: Vec<Vec<Rc<RefCell<BlockStat>>>>,
}

impl ProfilerData {
    pub(crate) fn new() -> ProfilerData {
        ProfilerData {
            main_start_time: Instant::now(),
            main_block: BlockStat::new(),
            blocks_stack: Vec::new(),
        }
    }

    pub(crate) fn build_report_string(&self) -> String {
        let mut report = String::with_capacity(8192);
        report += HTML_REPORT_HEADER;

        report += "<table>\n";
        report += "<thead><th>Block name</th><th>Global percents</th><th>Relative to parent percents</th><th>Average time</th></thead>\n";

        self.main_block.build_report().build_string(&mut report);

        report += "</table>\n";

        report += HTML_REPORT_FOOTER;
        report
    }
}
