use std::{
    time::Duration,
    collections::BTreeMap,
};

pub struct BlockStatReport {
    name: String,
    avg_time: Duration,
    global_percents: f32,
    relative_parent_percents: f32,
    children: Vec<BlockStatReport>,
}

impl BlockStatReport {
    pub fn build_string(&mut self, report: &mut String) {
        self.build_string_recurse(report, 0, 0)
    }

    fn build_string_recurse(&mut self, report: &mut String, depth: usize, max_name_len: usize) {
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
            child.build_string_recurse(report, depth + 1, max_name_len);
        }
    }
}


pub struct BlockStat {
    pub(crate) name: &'static str,
    pub(crate) total_time: Duration,
    pub(crate) measure_count: u32,
    pub(crate) children: BTreeMap<usize, BlockStat>,
}

impl BlockStat {
    pub fn new(name: &'static str) -> BlockStat {
        BlockStat {
            name,
            total_time: Duration::from_millis(0),
            measure_count: 0,
            children: BTreeMap::new(),
        }
    }

    pub fn build_report(&self) -> BlockStatReport {
        self.build_report_recurse(self.total_time, self.total_time, self.total_time, self.total_time)
    }

    fn build_report_recurse(&self, total_global_time: Duration, avg_global_time: Duration, total_parent_time: Duration, avg_parent_time: Duration) -> BlockStatReport {
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
                    stat.build_report_recurse(total_global_time, avg_global_time, total_parent_time, avg_parent_time)
                ).collect()
            },
        }
    }
}
