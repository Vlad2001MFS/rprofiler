use flume::{
    Sender, Receiver,
};

use std::{
    collections::BTreeMap,
    time::{
        Duration, Instant,
    },
    thread::ThreadId,
    rc::Rc,
    cell::RefCell,
    sync::Mutex,
};

#[macro_export]
macro_rules! profile_block {
    () => {
        let _profiler_block_guard = $crate::ProfilerBlockGuard::new({
            fn f() {}

            #[inline]
            fn type_name_of_val<T>(_: T) -> &'static str {
                std::any::type_name::<T>()
            }
            
            let name = type_name_of_val(f);
            &name[..name.len() - 3]
        });
    };
    (name $block_name:literal) => {
        let _profiler_block_guard = $crate::ProfilerBlockGuard::new($block_name);
    };
    (if_feature $name:literal) => {
        #[cfg(feature = $name)]
        let _profiler_block_guard = $crate::ProfilerBlockGuard::new({
            fn f() {}

            #[inline]
            fn type_name_of_val<T>(_: T) -> &'static str {
                std::any::type_name::<T>()
            }
            
            let name = type_name_of_val(f);
            &name[..name.len() - 3]
        });
    };
    (if_feature $feature_name:literal, name $block_name:literal) => {
        #[cfg(feature = $feature_name)]
        let _profiler_block_guard = $crate::ProfilerBlockGuard::new($block_name);
    };
}

struct BlockStatReport {
    name: String,
    avg_time: Duration,
    global_percents: f32,
    relative_parent_percents: f32,
    children: Vec<BlockStatReport>,
}

impl BlockStatReport {
    fn build_string(&mut self, report: &mut String, depth: usize, max_name_len: usize) {
        let name = self.name.replace("<", "&lt;").replace(">", "&gt;");

        *report += &format!(
            concat!(
                "<tr>",
                "<td style=\"padding-left: {}\">{}</td>",
                "<td>{:6.2} %</td>",
                "<td>{:6.2} %</td>",
                "<td>{:9.4} ms</td>",
                "</tr>\n"
            ),
            depth*25, name, self.global_percents, self.relative_parent_percents, self.avg_time.as_secs_f32()*1000.0
        );

        self.children.sort_by(|a, b| b.relative_parent_percents.partial_cmp(&a.relative_parent_percents).unwrap());

        for child in self.children.iter_mut() {
            child.build_string(report, depth + 1, max_name_len);
        }
    }
}

struct BlockStat {
    name: &'static str,
    total_time: Duration,
    measure_count: u32,
    children: BTreeMap<&'static str, Rc<RefCell<BlockStat>>>,
}

impl BlockStat {
    fn build_report(&self, total_global_time: Duration, avg_global_time: Duration, total_parent_time: Duration, avg_parent_time: Duration) -> BlockStatReport {
        let avg_time = self.total_time / self.measure_count;

        BlockStatReport {
            name: {
                let mut name = String::with_capacity(self.name.len());
                let mut name_parts_iter = self.name.split("::");
                while let Some(first_name_part) = name_parts_iter.next() {
                    match name_parts_iter.clone().next() {
                        Some(second_name_part) => {
                            let first_name_part_simplified = first_name_part.to_lowercase().replace("_", "");
                            let second_name_part_simplified = second_name_part.to_lowercase().replace("_", "");

                            match first_name_part_simplified == second_name_part_simplified {
                                true => {
                                    name += "::";
                                    name += second_name_part;
                                    name_parts_iter.next();
                                }
                                false => {
                                    name += "::";
                                    name += first_name_part;
                                }
                            }
                        }
                        None => {
                            name += "::";
                            name += first_name_part;
                        }
                    }
                }

                name.strip_prefix("::").map(|a| a.to_owned()).unwrap_or(name)
            },
            avg_time,
            global_percents: (self.total_time.as_secs_f32() / total_global_time.as_secs_f32())*100.0,
            relative_parent_percents: (self.total_time.as_secs_f32() / total_parent_time.as_secs_f32())*100.0,
            children: {
                let total_parent_time: Duration = self.total_time;
                let avg_parent_time: Duration = avg_time;
                self.children.iter().map(|(_, stat)|
                    stat.borrow().build_report(total_global_time, avg_global_time, total_parent_time, avg_parent_time)
                ).collect()
            },
        }
    }
}

pub struct ProfilerData {
    main_block: BlockStat,
    blocks_stack: Vec<Vec<Rc<RefCell<BlockStat>>>>,
}

impl ProfilerData {
    pub fn new() -> ProfilerData {
        ProfilerData {
            main_block: BlockStat {
                name: "",
                total_time: Duration::from_millis(0),
                measure_count: 0,
                children: BTreeMap::new(),
            },
            blocks_stack: Vec::new(),
        }
    }

    pub fn save_to_file(&self) {
        std::fs::write("./profile_info.html", self.build_report_string()).unwrap();
    }

    fn build_report_string(&self) -> String {
        let mut report = String::with_capacity(8192);
        report += r#"<html><body>
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

        report += "<table>\n";
        report += "<thead><th>Block name</th><th>Global percents</th><th>Relative to parent percents</th><th>Average time</th></thead>\n";

        let total_main_time = self.main_block.total_time;
        self.main_block.build_report(total_main_time, total_main_time, total_main_time, total_main_time).build_string(&mut report, 0, 0);

        report += "</table>\n";

        report += "</body></html>";
        report
    }
}

lazy_static! {
    pub static ref PROFILER: Profiler = Profiler::new();
}

enum ProfilerEvent {
    BeginMain,
    EndMain(Duration),
    BeginBlock {
        thread_id: ThreadId,
        name: &'static str,
    },
    EndBlock {
        thread_id: ThreadId,
        time: Duration,
    },
}

pub struct Profiler {
    main_start_time: Mutex<Instant>,
    events_sender: Sender<ProfilerEvent>,
    events_receiver: Receiver<ProfilerEvent>,
}

impl Profiler {
    pub fn process_events(&self, data: &mut ProfilerData) {
        profile_block!();

        for event in self.events_receiver.try_iter() {
            match event {
                ProfilerEvent::BeginMain => {
                    data.main_block.name = "rengine::run";
                },
                ProfilerEvent::EndMain(time) => {
                    data.main_block.total_time = time;
                    data.main_block.measure_count = 1;
                }
                ProfilerEvent::BeginBlock { thread_id, name } => {
                    let thread_id_value = unsafe { *(&thread_id as *const ThreadId as *const usize) };

                    if data.blocks_stack.len() < thread_id_value + 1 {
                        data.blocks_stack.resize(thread_id_value + 1, Vec::new());
                    }

                    let thread_blocks_stack = &mut data.blocks_stack[thread_id_value];

                    let block_stat = match thread_blocks_stack.last() {
                        Some(top_block_stat) => {
                            top_block_stat.borrow_mut().children.entry(name).or_insert_with(|| Rc::new(RefCell::new(BlockStat {
                                name,
                                total_time: Duration::from_millis(0),
                                measure_count: 0,
                                children: BTreeMap::new(),
                            }))).clone()
                        }
                        None => {
                            data.main_block.children.entry(name).or_insert_with(|| Rc::new(RefCell::new(BlockStat {
                                name,
                                total_time: Duration::from_millis(0),
                                measure_count: 0,
                                children: BTreeMap::new(),
                            }))).clone()
                        }
                    };
                    
                    thread_blocks_stack.push(block_stat);
                },
                ProfilerEvent::EndBlock { thread_id, time } => {
                    let thread_id_value = unsafe { *(&thread_id as *const ThreadId as *const usize) };
                    let thread_current_block = data.blocks_stack[thread_id_value].pop().unwrap();
                    thread_current_block.borrow_mut().total_time += time;
                    thread_current_block.borrow_mut().measure_count += 1;
                },
            }
        }
    }

    pub fn begin_main(&self) {
        *self.main_start_time.lock().unwrap() = Instant::now();
        self.events_sender.send(ProfilerEvent::BeginMain).unwrap();
    }

    pub fn end_main(&self) {
        let time = self.main_start_time.lock().unwrap().elapsed();
        self.events_sender.send(ProfilerEvent::EndMain(time)).unwrap();
    }

    fn new() -> Profiler {
        let (events_sender, events_receiver) = flume::unbounded();
        Profiler {
            main_start_time: Mutex::new(Instant::now()),
            events_sender,
            events_receiver,
        }
    }

    #[inline]
    fn begin_block(&self, name: &'static str) {
        self.events_sender.send(ProfilerEvent::BeginBlock {
            thread_id: std::thread::current().id(),
            name,
        }).unwrap();
    }

    #[inline]
    fn end_block(&self, time: Duration) {
        self.events_sender.send(ProfilerEvent::EndBlock {
            thread_id: std::thread::current().id(),
            time,
        }).unwrap();
    }
}

pub struct ProfilerBlockGuard {
    start_time: Instant,
}

impl ProfilerBlockGuard {
    #[inline]
    pub fn new(block_name: &'static str) -> ProfilerBlockGuard {
        let guard = ProfilerBlockGuard {
            start_time: Instant::now(),
        };
        PROFILER.begin_block(block_name);
        guard
    }
}

impl Drop for ProfilerBlockGuard {
    #[inline]
    fn drop(&mut self) {
        PROFILER.end_block(self.start_time.elapsed());
    }
}
