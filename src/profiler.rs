use crate::{
    BlockStat, ProfilerData,
};
use flume::{
    Sender, Receiver,
};
use std::{
    time::{
        Duration, Instant,
    },
    thread::ThreadId,
};

lazy_static! {
    pub static ref PROFILER: Profiler = Profiler::new();
}

#[cfg(not(feature = "disable_profiling"))]
enum ProfilerEvent {
    Initialize(Instant),
    Shutdown(Instant),
    ResetStats,
    BeginBlock {
        thread_id: ThreadId,
        name: &'static str,
    },
    EndBlock {
        thread_id: ThreadId,
        time: Duration,
    },
}

#[cfg(not(feature = "disable_profiling"))]
pub struct Profiler {
    events_sender: Sender<ProfilerEvent>,
    events_receiver: Receiver<ProfilerEvent>,
}

#[cfg(feature = "disable_profiling")]
pub struct Profiler;

#[cfg(not(feature = "disable_profiling"))]
impl Profiler {
    pub fn process_events(&self, data: &mut ProfilerData) {
        crate::profile_block!();

        for event in self.events_receiver.try_iter() {
            match event {
                ProfilerEvent::Initialize(time) => {
                    data.main_block_start_time = time;
                },
                ProfilerEvent::Shutdown(time) => {
                    data.main_block.total_time = time.duration_since(data.main_block_start_time);
                    data.main_block.measure_count = 1;
                }
                ProfilerEvent::ResetStats => data.reset_stats(),
                ProfilerEvent::BeginBlock { thread_id, name } => {
                    let name_hash = (name as *const str as *const u8) as usize;
                    let block_stat = match data.current_block_on_thread(thread_id) {
                        Some(top_block_stat) => {
                            let top_block_stat = unsafe { &mut *top_block_stat };
                            let block_stat = top_block_stat.children.entry(name_hash).or_insert_with(|| Box::new(BlockStat::new(name)));
                            block_stat.as_mut() as *mut _
                        },
                        None => {
                            let block_stat = data.main_block.children.entry(name_hash).or_insert_with(|| Box::new(BlockStat::new(name)));
                            block_stat.as_mut() as *mut _
                        },
                    };

                    data.push_block_to_thread_stack(thread_id, block_stat);
                },
                ProfilerEvent::EndBlock { thread_id, time } => {
                    let thread_current_block = data.pop_block_from_thread_stack(thread_id).unwrap();
                    let thread_current_block = unsafe { &mut *thread_current_block };
                    thread_current_block.total_time += time;
                    thread_current_block.measure_count += 1;
                },
            }
        }
    }

    pub fn initialize(&self) -> ProfilerData {
        self.events_sender.send(ProfilerEvent::Initialize(Instant::now())).unwrap();

        ProfilerData::new()
    }

    pub fn shutdown(&self, report_path: &str, profiler_data: &mut ProfilerData) {
        self.events_sender.send(ProfilerEvent::Shutdown(Instant::now())).unwrap();

        self.process_events(profiler_data);
        std::fs::write(report_path, profiler_data.build_report_string()).unwrap();
    }

    pub fn reset_stats(&self) {
        self.events_sender.send(ProfilerEvent::ResetStats).unwrap();
    }

    fn new() -> Profiler {
        let (events_sender, events_receiver) = flume::unbounded();
        Profiler {
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

#[cfg(feature = "disable_profiling")]
impl Profiler {
    pub fn process_events(&self, _data: &mut ProfilerData) {}

    pub fn initialize(&self) -> ProfilerData {
        ProfilerData::new()
    }

    pub fn shutdown(&self, _report_path: &str, _profiler_data: &mut ProfilerData) {}

    pub fn reset_stats(&self) {}

    fn new() -> Profiler {
        Profiler
    }

    #[inline]
    fn begin_block(&self, _name: &'static str) {}

    #[inline]
    fn end_block(&self, _time: Duration) {}
}

#[cfg(not(feature = "disable_profiling"))]
pub struct ProfilerBlockGuard {
    start_time: Instant,
}

#[cfg(feature = "disable_profiling")]
pub struct ProfilerBlockGuard;

#[cfg(feature = "disable_profiling")]
impl ProfilerBlockGuard {
    #[inline]
    pub fn new(_block_name: &'static str) -> ProfilerBlockGuard {
        ProfilerBlockGuard
    }
}

#[cfg(not(feature = "disable_profiling"))]
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

#[cfg(not(feature = "disable_profiling"))]
impl Drop for ProfilerBlockGuard {
    #[inline]
    fn drop(&mut self) {
        PROFILER.end_block(self.start_time.elapsed());
    }
}
