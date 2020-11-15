#[macro_use] extern crate lazy_static;
extern crate flume;

mod profiler;

pub use profiler::*;

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
        #[cfg(feature = $name)] profile_block!();
    };
    (if_feature $feature_name:literal, name $block_name:literal) => {
        #[cfg(feature = $feature_name)] profile_block!($block_name);
    };
}
