//! VM instruction handlers for process operations.
use vm::action::Action;
use vm::instruction::Instruction;
use vm::instructions::result::InstructionResult;
use vm::machine::Machine;

use compiled_code::RcCompiledCode;
use object_value;
use pools::PRIMARY_POOL;
use process::RcProcess;

/// Runs a CompiledCode in a new process.
///
/// This instruction takes 3 arguments:
///
/// 1. The register to store the PID in.
/// 2. A code objects index pointing to the CompiledCode object to run.
/// 3. The ID of the process pool to schedule the process on. Defaults to the ID
///    of the primary pool.
pub fn spawn_literal_process(machine: &Machine,
                             process: &RcProcess,
                             code: &RcCompiledCode,
                             instruction: &Instruction)
                             -> InstructionResult {
    let register = instruction.arg(0)?;
    let code_index = instruction.arg(1)?;
    let pool_id = instruction.arg(2).unwrap_or(PRIMARY_POOL);
    let code_obj = code.code_object(code_index)?;

    machine.spawn_process(process, pool_id, code_obj, register)?;

    Ok(Action::None)
}

/// Runs a CompiledCode in a new process using a runtime allocated
/// CompiledCode.
///
/// This instruction takes the same arguments as the "spawn_literal_process"
/// instruction except instead of a code object index the 2nd argument
/// should point to a register containing a CompiledCode object.
pub fn spawn_process(machine: &Machine,
                     process: &RcProcess,
                     _: &RcCompiledCode,
                     instruction: &Instruction)
                     -> InstructionResult {
    let register = instruction.arg(0)?;
    let code_ptr = process.get_register(instruction.arg(1)?)?;

    let pool_id = if let Ok(pool_reg) = instruction.arg(2) {
        let ptr = process.get_register(pool_reg)?;

        ptr.get().value.as_integer()? as usize
    } else {
        PRIMARY_POOL
    };

    let code_obj = code_ptr.get().value.as_compiled_code()?;

    machine.spawn_process(process, pool_id, code_obj, register)?;

    Ok(Action::None)
}

/// Sends a message to a process.
///
/// This instruction takes 3 arguments:
///
/// 1. The register to store the message in.
/// 2. The register containing the PID to send the message to.
/// 3. The register containing the message (an object) to send to the
///    process.
pub fn send_process_message(machine: &Machine,
                            process: &RcProcess,
                            _: &RcCompiledCode,
                            instruction: &Instruction)
                            -> InstructionResult {
    let register = instruction.arg(0)?;
    let pid_ptr = process.get_register(instruction.arg(1)?)?;
    let msg_ptr = process.get_register(instruction.arg(2)?)?;
    let pid = pid_ptr.get().value.as_integer()? as usize;

    if let Some(receiver) = read_lock!(machine.state.process_table).get(&pid) {
        receiver.send_message(&process, msg_ptr);
    }

    process.set_register(register, msg_ptr);

    Ok(Action::None)
}

/// Receives a message for the current process.
///
/// This instruction takes 1 argument: the register to store the resulting
/// message in.
///
/// If no messages are available the current process will be suspended, and
/// the instruction will be retried the next time the process is executed.
pub fn receive_process_message(_: &Machine,
                               process: &RcProcess,
                               _: &RcCompiledCode,
                               instruction: &Instruction)
                               -> InstructionResult {
    let register = instruction.arg(0)?;
    let result = if let Some(msg_ptr) = process.receive_message() {
        process.set_register(register, msg_ptr);

        Action::None
    } else {
        Action::Suspend
    };

    Ok(result)
}

/// Gets the PID of the currently running process.
///
/// This instruction requires one argument: the register to store the PID
/// in (as an integer).
pub fn get_current_pid(machine: &Machine,
                       process: &RcProcess,
                       _: &RcCompiledCode,
                       instruction: &Instruction)
                       -> InstructionResult {
    let register = instruction.arg(0)?;
    let pid = process.pid;

    let pid_obj = process.allocate(object_value::integer(pid as i64),
                                   machine.state.integer_prototype.clone());

    process.set_register(register, pid_obj);

    Ok(Action::None)
}
