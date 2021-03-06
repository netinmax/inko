//! VM instruction handlers for array operations.
use immix::copy_object::CopyObject;

use vm::action::Action;
use vm::instruction::Instruction;
use vm::instructions::result::InstructionResult;
use vm::machine::Machine;

use compiled_code::RcCompiledCode;
use object_value;
use process::RcProcess;

/// Returns a vector index for an i64
macro_rules! int_to_vector_index {
    ($vec: expr, $index: expr) => ({
        if $index >= 0 as i64 {
            $index as usize
        }
        else {
            ($vec.len() as i64 - $index) as usize
        }
    });
}

/// Ensures the given index is within the bounds of the array.
macro_rules! ensure_array_within_bounds {
    ($array: ident, $index: expr) => (
        if $index >= $array.len() {
            return Err(format!("array index {} is out of bounds", $index));
        }
    );
}

/// Sets an array in a register.
///
/// This instruction requires at least one argument: the register to store
/// the resulting array in. Any extra instruction arguments should point to
/// registers containing objects to store in the array.
pub fn set_array(machine: &Machine,
                 process: &RcProcess,
                 _: &RcCompiledCode,
                 instruction: &Instruction)
                 -> InstructionResult {
    let register = instruction.arg(0)?;
    let val_count = instruction.arguments.len() - 1;

    let values =
        machine.collect_arguments(process.clone(), instruction, 1, val_count)?;

    let obj = process.allocate(object_value::array(values),
                               machine.state.array_prototype);

    process.set_register(register, obj);

    Ok(Action::None)
}

/// Inserts a value in an array.
///
/// This instruction requires 4 arguments:
///
/// 1. The register to store the result (the inserted value) in.
/// 2. The register containing the array to insert into.
/// 3. The register containing the index (as an integer) to insert at.
/// 4. The register containing the value to insert.
///
/// An error is returned when the index is greater than the array length. A
/// negative index can be used to indicate a position from the end of the
/// array.
pub fn array_insert(machine: &Machine,
                    process: &RcProcess,
                    _: &RcCompiledCode,
                    instruction: &Instruction)
                    -> InstructionResult {
    let register = instruction.arg(0)?;
    let array_ptr = process.get_register(instruction.arg(1)?)?;
    let index_ptr = process.get_register(instruction.arg(2)?)?;
    let value_ptr = process.get_register(instruction.arg(3)?)?;

    let mut array = array_ptr.get_mut();
    let index_obj = index_ptr.get();

    let mut vector = array.value.as_array_mut()?;
    let index = int_to_vector_index!(vector, index_obj.value.as_integer()?);

    ensure_array_within_bounds!(vector, index);

    let value = copy_if_permanent!(machine.state.permanent_allocator,
                                   value_ptr,
                                   array_ptr);

    if vector.get(index).is_some() {
        vector[index] = value;
    } else {
        vector.insert(index, value);
    }

    process.set_register(register, value);

    Ok(Action::None)
}

/// Gets the value of an array index.
///
/// This instruction requires 3 arguments:
///
/// 1. The register to store the value in.
/// 2. The register containing the array.
/// 3. The register containing the index.
///
/// An error is returned when the index is greater than the array length. A
/// negative index can be used to indicate a position from the end of the
/// array.
pub fn array_at(_: &Machine,
                process: &RcProcess,
                _: &RcCompiledCode,
                instruction: &Instruction)
                -> InstructionResult {
    let register = instruction.arg(0)?;
    let array_ptr = process.get_register(instruction.arg(1)?)?;
    let index_ptr = process.get_register(instruction.arg(2)?)?;
    let array = array_ptr.get();

    let index_obj = index_ptr.get();
    let vector = array.value.as_array()?;
    let index = int_to_vector_index!(vector, index_obj.value.as_integer()?);

    ensure_array_within_bounds!(vector, index);

    let value = vector[index].clone();

    process.set_register(register, value);

    Ok(Action::None)
}

/// Removes a value from an array.
///
/// This instruction requires 3 arguments:
///
/// 1. The register to store the removed value in.
/// 2. The register containing the array to remove a value from.
/// 3. The register containing the index.
///
/// An error is returned when the index is greater than the array length. A
/// negative index can be used to indicate a position from the end of the
/// array.
pub fn array_remove(_: &Machine,
                    process: &RcProcess,
                    _: &RcCompiledCode,
                    instruction: &Instruction)
                    -> InstructionResult {
    let register = instruction.arg(0)?;
    let array_ptr = process.get_register(instruction.arg(1)?)?;
    let index_ptr = process.get_register(instruction.arg(2)?)?;

    let mut array = array_ptr.get_mut();
    let index_obj = index_ptr.get();
    let mut vector = array.value.as_array_mut()?;
    let index = int_to_vector_index!(vector, index_obj.value.as_integer()?);

    ensure_array_within_bounds!(vector, index);

    let value = vector.remove(index);

    process.set_register(register, value);

    Ok(Action::None)
}

/// Gets the amount of elements in an array.
///
/// This instruction requires 2 arguments:
///
/// 1. The register to store the length in.
/// 2. The register containing the array.
pub fn array_length(machine: &Machine,
                    process: &RcProcess,
                    _: &RcCompiledCode,
                    instruction: &Instruction)
                    -> InstructionResult {
    let register = instruction.arg(0)?;
    let array_ptr = process.get_register(instruction.arg(1)?)?;
    let array = array_ptr.get();
    let vector = array.value.as_array()?;
    let length = vector.len() as i64;

    let obj = process.allocate(object_value::integer(length),
                               machine.state.integer_prototype.clone());

    process.set_register(register, obj);

    Ok(Action::None)
}

/// Removes all elements from an array.
///
/// This instruction requires 1 argument: the register of the array.
pub fn array_clear(_: &Machine,
                   process: &RcProcess,
                   _: &RcCompiledCode,
                   instruction: &Instruction)
                   -> InstructionResult {
    let array_ptr = process.get_register(instruction.arg(0)?)?;
    let mut array = array_ptr.get_mut();
    let mut vector = array.value.as_array_mut()?;

    vector.clear();

    Ok(Action::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use object_value;
    use vm::instructions::test::*;
    use vm::instruction::InstructionType;

    mod set_array {
        use super::*;

        #[test]
        fn test_without_arguments() {
            let (machine, code, process) = setup();

            let instruction = new_instruction(InstructionType::SetArray,
                                              Vec::new());

            assert!(set_array(&machine, &process, &code, &instruction).is_err());
        }

        #[test]
        fn test_with_valid_arguments() {
            let (machine, code, process) = setup();

            let instruction = new_instruction(InstructionType::SetArray, vec![0]);

            let result = set_array(&machine, &process, &code, &instruction);

            assert!(result.is_ok());

            let pointer = process.get_register(0).unwrap();
            let object = pointer.get();

            assert!(object.value.is_array());
            assert!(object.prototype == machine.state.array_prototype);
        }

        #[test]
        fn test_with_multiple_valid_arguments() {
            let (machine, code, process) = setup();

            let instruction = new_instruction(InstructionType::SetArray,
                                              vec![2, 0, 1]);

            let value1 = process.allocate_empty();
            let value2 = process.allocate_empty();

            process.set_register(0, value1);
            process.set_register(1, value2);

            let result = set_array(&machine, &process, &code, &instruction);

            assert!(result.is_ok());

            let pointer = process.get_register(2).unwrap();
            let object = pointer.get();

            assert!(object.value.is_array());

            let values = object.value.as_array().unwrap();

            assert_eq!(values.len(), 2);

            assert!(values[0] == value1);
            assert!(values[1] == value2);
        }
    }

    mod array_insert {
        use super::*;

        #[test]
        fn test_without_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayInsert,
                                              Vec::new());

            let result = array_insert(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_without_array_argument() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayInsert,
                                              vec![3]);

            let result = array_insert(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_without_index_argument() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayInsert,
                                              vec![3, 0]);

            let result = array_insert(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_without_value_index() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayInsert,
                                              vec![3, 0, 1]);

            let result = array_insert(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_undefined_registers() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayInsert,
                                              vec![3, 0, 1, 2]);

            let result = array_insert(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }


        #[test]
        fn test_with_valid_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayInsert,
                                              vec![3, 0, 1, 2]);

            let array = process
                .allocate_without_prototype(object_value::array(Vec::new()));

            let index =
                process.allocate_without_prototype(object_value::integer(0));

            let value =
                process.allocate_without_prototype(object_value::integer(5));

            process.set_register(0, array);
            process.set_register(1, index);
            process.set_register(2, value);

            let result = array_insert(&machine, &process, &code, &instruction);

            assert!(result.is_ok());

            let pointer = process.get_register(3).unwrap();
            let object = pointer.get();

            assert!(object.value.is_integer());
            assert_eq!(object.value.as_integer().unwrap(), 5);
        }
    }

    mod array_at {
        use super::*;

        #[test]
        fn test_without_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayAt,
                                              Vec::new());

            let result = array_at(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_without_array_argument() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayAt, vec![2]);
            let result = array_at(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_without_index_argument() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayAt,
                                              vec![2, 0]);

            let result = array_at(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_undefined_registers() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayAt,
                                              vec![2, 0, 1]);

            let result = array_at(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_valid_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayAt,
                                              vec![2, 0, 1]);

            let value =
                process.allocate_without_prototype(object_value::integer(5));

            let array = process
                .allocate_without_prototype(object_value::array(vec![value]));

            let index =
                process.allocate_without_prototype(object_value::integer(0));

            process.set_register(0, array);
            process.set_register(1, index);

            let result = array_at(&machine, &process, &code, &instruction);

            assert!(result.is_ok());

            let pointer = process.get_register(2).unwrap();
            let object = pointer.get();

            assert!(object.value.is_integer());
            assert_eq!(object.value.as_integer().unwrap(), 5);
        }
    }

    mod array_remove {
        use super::*;

        #[test]
        fn test_without_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayRemove,
                                              Vec::new());

            let result = array_remove(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_without_array_argument() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayRemove,
                                              vec![2]);

            let result = array_remove(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_without_index_argument() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayRemove,
                                              vec![2, 0]);

            let result = array_remove(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_undefined_registers() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayRemove,
                                              vec![2, 0, 1]);

            let result = array_remove(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_valid_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayRemove,
                                              vec![2, 0, 1]);

            let value =
                process.allocate_without_prototype(object_value::integer(5));

            let array = process
                .allocate_without_prototype(object_value::array(vec![value]));

            let index =
                process.allocate_without_prototype(object_value::integer(0));

            process.set_register(0, array);
            process.set_register(1, index);

            let result = array_remove(&machine, &process, &code, &instruction);

            assert!(result.is_ok());

            let removed_pointer = process.get_register(2).unwrap();
            let removed_object = removed_pointer.get();

            assert!(removed_object.value.is_integer());
            assert_eq!(removed_object.value.as_integer().unwrap(), 5);

            assert_eq!(array.get().value.as_array().unwrap().len(), 0);
        }
    }

    mod array_length {
        use super::*;

        #[test]
        fn test_without_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayLength,
                                              Vec::new());

            let result = array_length(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_without_array_argument() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayLength,
                                              vec![1]);

            let result = array_length(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_undefined_registers() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayLength,
                                              vec![1, 0]);

            let result = array_length(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_valid_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayLength,
                                              vec![1, 0]);

            let value = process.allocate_empty();

            let array = process
                .allocate_without_prototype(object_value::array(vec![value]));

            process.set_register(0, array);

            let result = array_length(&machine, &process, &code, &instruction);

            assert!(result.is_ok());

            let pointer = process.get_register(1).unwrap();
            let object = pointer.get();

            assert!(object.value.is_integer());
            assert_eq!(object.value.as_integer().unwrap(), 1);
        }
    }

    mod array_clear {
        use super::*;

        #[test]
        fn test_without_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayClear,
                                              Vec::new());

            let result = array_clear(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_undefined_registers() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayClear,
                                              vec![0]);

            let result = array_clear(&machine, &process, &code, &instruction);

            assert!(result.is_err());
        }

        #[test]
        fn test_with_valid_arguments() {
            let (machine, code, process) = setup();
            let instruction = new_instruction(InstructionType::ArrayClear,
                                              vec![0]);

            let value = process.allocate_empty();

            let array = process
                .allocate_without_prototype(object_value::array(vec![value]));

            process.set_register(0, array);

            let result = array_clear(&machine, &process, &code, &instruction);

            assert!(result.is_ok());

            let object = array.get();

            assert_eq!(object.value.as_array().unwrap().len(), 0);
        }
    }
}
