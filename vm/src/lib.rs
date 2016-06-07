#![feature(alloc_system)]
extern crate alloc_system;

pub mod macros;

pub mod binding;
pub mod bytecode_parser;
pub mod call_frame;
pub mod compiled_code;
pub mod config;
pub mod errors;
pub mod heap;
pub mod inbox;
pub mod instruction;
pub mod object;
pub mod object_header;
pub mod object_pointer;
pub mod object_value;
pub mod register;
pub mod process;
pub mod process_list;
pub mod thread;
pub mod thread_list;
pub mod virtual_machine;
pub mod virtual_machine_methods;
pub mod virtual_machine_result;
