use register::Register;
use variable_scope::VariableScope;

pub struct CallFrame<'l> {
    pub name: &'l str,
    pub file: &'l str,
    pub line: usize,
    pub parent: Option<Box<CallFrame<'l>>>,
    pub register: Register<'l>,
    pub variables: VariableScope<'l>
}

impl<'l> CallFrame<'l> {
    pub fn new(name: &'l str, file: &'l str, line: usize) -> CallFrame<'l> {
        let frame = CallFrame {
            name: name,
            file: file,
            line: line,
            parent: Option::None,
            register: Register::new(),
            variables: VariableScope::new()
        };

        frame
    }

    pub fn set_parent(&mut self, parent: CallFrame<'l>) {
        self.parent = Option::Some(Box::new(parent));
    }
}
