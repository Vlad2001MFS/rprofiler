use crate::BlockStat;
use std::{
    time::Instant,
    thread::ThreadId,
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

#[inline]
fn thread_id_to_usize(thread_id: ThreadId) -> usize {
    unsafe { *(&thread_id as *const ThreadId as *const usize) }
}

pub struct ProfilerData {
    pub(crate) main_block_start_time: Instant,
    pub(crate) main_block: BlockStat,
    pub(crate) blocks_stack: Vec<Vec<*mut BlockStat>>,
}

impl ProfilerData {
    pub(crate) fn new() -> ProfilerData {
        ProfilerData {
            main_block_start_time: Instant::now(),
            main_block: BlockStat::new("ProfilerMainBlock"),
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

    #[inline]
    pub(crate) fn current_block_on_thread(&self, thread_id: ThreadId) -> Option<*mut BlockStat> {
        let thread_id_value = thread_id_to_usize(thread_id);
        self.blocks_stack.get(thread_id_value).and_then(|a| a.last().cloned())
    }

    #[inline]
    pub(crate) fn push_block_to_thread_stack(&mut self, thread_id: ThreadId, block: *mut BlockStat) {
        let thread_id_value = thread_id_to_usize(thread_id);

        if self.blocks_stack.len() < thread_id_value + 1 {
            self.blocks_stack.resize(thread_id_value + 1, Vec::new());
        }

        unsafe {
            self.blocks_stack.get_unchecked_mut(thread_id_value).push(block);
        }
    }

    #[inline]
    pub(crate) fn pop_block_from_thread_stack(&mut self, thread_id: ThreadId) -> Option<*mut BlockStat> {
        let thread_id_value = thread_id_to_usize(thread_id);
        self.blocks_stack.get_mut(thread_id_value).and_then(|a| a.pop())
    }
}
