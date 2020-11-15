use crate::{
    BlockStat, BlockStatReport, ProfilerData,
};
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
        for event in self.events_receiver.try_iter() {
            match event {
                ProfilerEvent::BeginMain => {
                    data.main_block.name = "ProfilerMainBlock";
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

    pub fn initialize(&self) -> ProfilerData {
        *self.main_start_time.lock().unwrap() = Instant::now();
        self.events_sender.send(ProfilerEvent::BeginMain).unwrap();

        ProfilerData::new()
    }

    pub fn shutdown(&self, report_path: &str, profiler_data: &mut ProfilerData) {
        let time = self.main_start_time.lock().unwrap().elapsed();
        self.events_sender.send(ProfilerEvent::EndMain(time)).unwrap();
        self.process_events(profiler_data);
        std::fs::write(report_path, profiler_data.build_report_string()).unwrap();
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
