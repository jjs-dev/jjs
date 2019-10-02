//! This module defines templates
//! for CMakeLists.txt files

static CHECKER_TPL: &str = include_str!("checker_tpl.cmake");

pub struct CheckerOptions {}

pub fn get_checker_cmakefile(_options: CheckerOptions) -> String {
    CHECKER_TPL.to_string()
}
